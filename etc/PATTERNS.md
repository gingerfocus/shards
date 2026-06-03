# SPEC: Pattern Detection (SIMD & GPU)

## Overview

The pattern detector analyzes memory access traces from the RISC-V
interpreter and IR structures from the shell DSL to identify regions
eligible for SIMD vectorization or GPU offloading.

It operates at two levels:
1. **Per-page**: Uses `PageAccessSummary` (stride histogram, access pattern)
2. **Global**: Uses basic block execution counts + memory access traces

## Detection Pipeline

```
  PageAccessSummary[]              HotBlockInfo[]
        │                               │
        ▼                               ▼
┌─────────────────┐           ┌──────────────────┐
│ Stride Analyzer │           │ Loop Detector     │
│ per-page stride │           │ backedges → loops │
│ classification │           │ loop nesting      │
└────────┬────────┘           └────────┬─────────┘
         │                             │
         └──────────────┬──────────────┘
                        ▼
              ┌──────────────────────┐
              │ Loop Body Extractor   │
              │ traces per iteration  │
              └──────────┬───────────┘
                         ▼
              ┌──────────────────────┐
              │ MapLoop Classifier    │
              │ purity check         │
              │ stride-1 contiguity  │
              │ side-effect analysis │
              └──────────┬───────────┘
                         ▼
              ┌──────────────────────┐
              │ Lowering Decision     │
              │ scalar / simd / gpu  │
              └──────────────────────┘
```

## 1. Loop Detection

Identify loops from the execution trace by detecting backward branches
(PC target < PC source).

```rust
struct LoopInfo {
    header_pc:     u64,
    latch_pc:      u64,       // last PC before backedge
    body_pcs:      HashSet<u64>,
    iteration_count: u64,
    is_inner_loop: bool,
    parent_loop:   Option<u64>, // header PC of outer loop
}

fn detect_loops(traces: &[InsnTrace]) -> Vec<LoopInfo> {
    let mut loops = Vec::new();
    let mut seen_backedges = HashSet::new();

    for i in 1..traces.len() {
        let prev_pc = traces[i - 1].pc;
        let curr_pc = traces[i].pc;

        // Backedge: we jumped to an earlier PC
        if curr_pc < prev_pc && !seen_backedges.contains(&(prev_pc, curr_pc)) {
            seen_backedges.insert((prev_pc, curr_pc));

            // Body is all PCs between header and latch
            let body = traces.iter()
                .filter(|t| t.pc >= curr_pc && t.pc <= prev_pc)
                .map(|t| t.pc)
                .collect();

            loops.push(LoopInfo {
                header_pc: curr_pc,
                latch_pc: prev_pc,
                body_pcs: body,
                iteration_count: 0,
                is_inner_loop: false,
                parent_loop: None,
            });
        }
    }

    // Nested loop analysis
    for i in 0..loops.len() {
        for j in 0..loops.len() {
            if i != j
                && loops[j].header_pc > loops[i].header_pc
                && loops[j].latch_pc < loops[i].latch_pc
            {
                loops[j].is_inner_loop = true;
                loops[j].parent_loop = Some(loops[i].header_pc);
            }
        }
    }

    loops
}
```

## 2. Loop Body Analysis

Extract the memory access pattern for one iteration of a loop.

```rust
struct LoopBodyPattern {
    header_pc:      u64,
    accesses:       Vec<BodyAccess>,
    input_arrays:   Vec<ArrayDescriptor>,
    output_arrays:  Vec<ArrayDescriptor>,

    is_pure:        bool,
    is_element_wise: bool,
    stride_analysis: HashMap<u64, StrideInfo>,
}

struct BodyAccess {
    pc:       u64,
    addr:     u64,
    kind:     AccessKind,
    size:     u8,
    array_base: Option<u64>,  // which array this belongs to
}

struct ArrayDescriptor {
    base_reg:    u8,          // RISC-V register holding base address
    base_addr:   u64,         // resolved base address
    element_size: u8,
    element_count: u64,
    access_pattern: AccessPattern,
}

struct StrideInfo {
    stride:     i64,
    confidence: f64,   // 0.0 to 1.0
    contiguous: bool,  // stride == element_size
}
```

```rust
fn analyze_loop_body(
    traces: &[InsnTrace],
    loop_info: &LoopInfo,
    page_summaries: &HashMap<u64, PageAccessSummary>,
) -> LoopBodyPattern {
    let mut body = LoopBodyPattern {
        header_pc: loop_info.header_pc,
        accesses: Vec::new(),
        input_arrays: Vec::new(),
        output_arrays: Vec::new(),
        is_pure: true,
        is_element_wise: true,
        stride_analysis: HashMap::new(),
    };

    // Extract one iteration from the trace
    let mut in_body = false;
    let mut prev_addr = None;

    for trace in traces {
        if trace.pc == loop_info.header_pc {
            if in_body { break; }  // we've seen a full iteration
            in_body = true;
        }

        if !in_body { continue; }

        if let Some(mem) = &trace.mem_access {
            body.accesses.push(BodyAccess {
                pc: trace.pc,
                addr: mem.addr,
                kind: mem.kind,
                size: mem.size,
                array_base: None,  // filled in later
            });

            // Stride tracking
            if let Some(prev) = prev_addr {
                let stride = mem.addr as i64 - prev as i64;
                let entry = body.stride_analysis
                    .entry(mem.addr & !0xFFF) // group by page
                    .or_insert(StrideInfo {
                        stride, confidence: 0.0,
                        contiguous: false,
                    });
                entry.stride = stride;
            }
            prev_addr = Some(mem.addr);

            // Purity check
            if trace.opcode & 0x7F == 0x73 { // SYSTEM opcode
                body.is_pure = false;         // ecall/ebreak in loop body
            }
        }
    }

    // Map accesses to arrays
    let mut base_addrs = HashMap::new();
    for acc in &body.accesses {
        let base = acc.addr & !(acc.size as u64 - 1); // align down
        let entry = base_addrs.entry(base).or_insert(ArrayDescriptor {
            base_reg: 0,
            base_addr: base,
            element_size: acc.size,
            element_count: 0,
            access_pattern: AccessPattern::Unknown,
        });
        entry.element_count += 1;
    }

    body.input_arrays = base_addrs.into_values().collect();

    // Check if element-wise: all accesses are to known arrays, no
    // inter-element dependencies
    body.is_element_wise = body.input_arrays.len() <= 4
        && !has_cross_iteration_dependency(&body.accesses);

    body
}
```

## 3. MapLoop Classification

Determine if a loop can be vectorized.

```rust
fn classify_map_loop(body: &LoopBodyPattern) -> Option<MapLoopCandidate> {
    if !body.is_pure { return None; }
    if !body.is_element_wise { return None; }

    // All input arrays must be stride-1 contiguous
    for arr in &body.input_arrays {
        let page_id = arr.base_addr >> 12;
        // Check page summary for stride-1 dominance
    }

    // No output-to-input aliasing within one iteration
    if aliases_exist(&body.output_arrays, &body.input_arrays) {
        return None;
    }

    Some(MapLoopCandidate {
        header_pc: body.header_pc,
        element_type: body.output_arrays[0].element_size, // infer from store size
        element_count: body.output_arrays[0].element_count,
        output_array_base: body.output_arrays[0].base_addr,
        input_array_bases: body.input_arrays.iter().map(|a| a.base_addr).collect(),
    })
}

struct MapLoopCandidate {
    header_pc:         u64,
    element_type:      u8,      // element size in bytes
    element_count:     u64,
    output_array_base: u64,
    input_array_bases: Vec<u64>,
}
```

## 4. Stride Classification from Page Summaries

```rust
fn classify_strides(summary: &PageAccessSummary, element_size: u8) -> StrideClassification {
    let total: u64 = summary.stride_histogram.values().sum();
    if total == 0 {
        return StrideClassification::Insufficient;
    }

    let target_stride = element_size as i64;

    // Check for stride == element_size (contiguous, stride-1 access)
    let stride1_count = summary.stride_histogram
        .get(&target_stride)
        .copied()
        .unwrap_or(0);

    let stride1_pct = stride1_count as f64 / total as f64;

    if stride1_pct > 0.95 {
        return StrideClassification::Contiguous;
    }

    // Check for regular non-unit stride
    let dominant = summary.stride_histogram.iter()
        .max_by_key(|(_, c)| *c)
        .unwrap();

    let dominant_pct = *dominant.1 as f64 / total as f64;

    if dominant_pct > 0.8 && *dominant.0 > 0 {
        return StrideClassification::RegularStride(*dominant.0);
    }

    // Check for broadcast (stride 0 dominates)
    let zero_count = summary.stride_histogram.get(&0).copied().unwrap_or(0);
    if zero_count as f64 / total as f64 > 0.9 {
        return StrideClassification::Broadcast;
    }

    if total > 100 {
        // Enough data, pattern is genuinely irregular
        return StrideClassification::Irregular;
    }

    StrideClassification::Insufficient
}

enum StrideClassification {
    Contiguous,              // stride == element_size, great for SIMD
    RegularStride(i64),      // constant non-unit stride, OK for SIMD gather
    Broadcast,               // same address, great for SIMD broadcast
    Irregular,               // no clear pattern, CPU scalar or GPU
    Insufficient,            // need more trace data
}
```

## 5. Lowering Decision Engine

```rust
struct LoweringDecision {
    region:       (u64, u64),     // start/end PC range
    kind:         LoweringKind,
    reason:       String,
    element_count: u64,
    simd_width:   u32,            // e.g. 4 for f64 on AVX2, 2 for f64 on NEON
    workgroup_size: u32,          // for GPU
}

enum LoweringKind {
    Scalar,       // Cranelift scalar JIT
    Simd,         // Cranelift SIMD (AVX2/NEON)
    Gpu,          // wgpu compute shader
    Interpreted,  // stay in interpreter
}

fn decide_lowering(
    candidate: &MapLoopCandidate,
    isa_features: &IsaFeatures,
    execution_count: u64,
) -> LoweringDecision {
    let elements = candidate.element_count;
    let elem_size = candidate.element_type;

    // Cold code: stay interpreted
    if execution_count < 100 {
        return LoweringDecision {
            region: (candidate.header_pc, candidate.header_pc),
            kind: LoweringKind::Interpreted,
            reason: "cold".into(),
            element_count: elements,
            simd_width: 0,
            workgroup_size: 0,
        };
    }

    let simd_reg_bytes = isa_features.simd_register_bytes(); // 16 SSE, 32 AVX2, 64 AVX512
    let simd_width = simd_reg_bytes / elem_size as u32;

    // Small: scalar or SIMD
    if elements < 1_000 {
        return LoweringDecision {
            region: (candidate.header_pc, candidate.header_pc),
            kind: LoweringKind::Scalar,
            reason: "small_count".into(),
            element_count: elements,
            simd_width: 0,
            workgroup_size: 0,
        };
    }

    if elements < 10_000 {
        return LoweringDecision {
            region: (candidate.header_pc, candidate.header_pc),
            kind: LoweringKind::Simd,
            reason: format!("medium_count, simd_width={}", simd_width),
            element_count: elements,
            simd_width,
            workgroup_size: 0,
        };
    }

    // Large: GPU if available
    LoweringDecision {
        region: (candidate.header_pc, candidate.header_pc),
        kind: LoweringKind::Gpu,
        reason: format!("large_count={}", elements),
        element_count: elements,
        simd_width,
        workgroup_size: 256.min(elements as u32),
    }
}
```

## 6. RISC-V Trace → MapLoop Candidate

Bridges the RISC-V execution traces to the shared IR's `MapLoop` format.

```rust
fn trace_to_map_loop(
    candidate: &MapLoopCandidate,
    traces: &[InsnTrace],
) -> MapLoop {
    let mut body = Vec::new();

    // Extract one iteration's worth of instructions
    let iter_traces = extract_one_iteration(traces, candidate.header_pc);

    // Map RISC-V registers to IR Locals
    let mut reg_to_local: HashMap<u8, Local> = HashMap::new();

    for trace in &iter_traces {
        if let Some(mem) = &trace.mem_access {
            // Classify: which IR array does this correspond to?
            let array_local = classify_array(mem.addr, candidate);
            let ir_op = riscv_mem_to_ir(trace, mem, array_local, &mut reg_to_local);
            body.push(ir_op);
        } else {
            let ir_op = riscv_alu_to_ir(trace, &mut reg_to_local);
            body.push(ir_op);
        }
    }

    MapLoop {
        iterator: reg_to_local[&ITERATOR_REG],
        start: ConstInt(0),
        end: ConstInt(candidate.element_count),
        kind: LoopKind::RangeExclusive,
        element_type: size_to_scalar_type(candidate.element_type),
        body,
        output_array: reg_to_local[&OUTPUT_BASE_REG],
        input_arrays: candidate.input_array_bases.iter()
            .map(|_| todo!())
            .collect(),
    }
}
```

## 7. IR-Level MapLoop Detection (DSL Path)

When the source is the shell DSL (not RISC-V), loop detection is trivial
because `MapLoop` nodes are explicitly constructed by the parser. The
detector just validates:

1. The body contains only `allowed_ops` (no side effects)
2. The output array is not aliased with any input array
3. The element count is deterministic (known at compile time OR runtime constant)

```rust
fn validate_dsl_map_loop(mloop: &MapLoop) -> Result<(), Vec<PatternError>> {
    let mut errors = Vec::new();

    for op in &mloop.body {
        if !is_allowed_op(op) {
            errors.push(PatternError::ForbiddenOp(op.clone()));
        }
    }

    if mloop.input_arrays.contains(&mloop.output_array) {
        errors.push(PatternError::AliasedOutput);
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn is_allowed_op(op: &IrOp) -> bool {
    matches!(op,
        IrOp::Add { .. } | IrOp::Sub { .. } | IrOp::Mul { .. } |
        IrOp::Div { .. } | IrOp::Load { .. } | IrOp::Store { .. } |
        IrOp::ArrayIndex { .. } | IrOp::ConstInt { .. } | IrOp::ConstFlt { .. }
    )
}
```

## ISA Feature Detection

```rust
struct IsaFeatures {
    simd_register_width: u32,  // bytes: 16 (SSE), 32 (AVX2), 64 (AVX-512)
    has_avx2:           bool,
    has_avx512f:        bool,
    has_neon:           bool,
    has_sve:            bool,
}

impl IsaFeatures {
    fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            let cpuid = raw_cpuid::CpuId::new();
            let features = cpuid.get_feature_info().unwrap();
            let ext = cpuid.get_extended_feature_info().unwrap();
            IsaFeatures {
                simd_register_width: if ext.has_avx512f() { 64 }
                                     else if ext.has_avx2() { 32 }
                                     else { 16 },
                has_avx2: ext.has_avx2(),
                has_avx512f: ext.has_avx512f(),
                has_neon: false,
                has_sve: false,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            IsaFeatures {
                simd_register_width: 16, // NEON 128-bit, SVE varies
                has_avx2: false,
                has_avx512f: false,
                has_neon: true,
                has_sve: std::arch::is_aarch64_feature_detected!("sve"),
            }
        }
    }

    fn simd_register_bytes(&self) -> u32 { self.simd_register_width }
}
```
