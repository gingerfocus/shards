# ShardVM — Project Plan

## Vision

A cloud execution platform where users submit RISC-V programs that are
interpreted, profiled for memory access patterns, and dynamically lowered
to CPU SIMD or GPU compute. The runtime state (registers, memory pages,
lowering records) is checkpointable and migratable between cloud nodes.

Two input sources share the same lowering pipeline:
- A shell/DSL language (parsed via Rush lexer, produces shared IR)
- Raw RISC-V RV64G + RVV binaries (interpreted + traced)

## Big TODOs

### Phase 1: Shared IR + Scalar JIT (Shell DSL)
- [ ] Create `crates/shardsimd/` crate (binary)
- [ ] Define shared mid-level IR (`ir.rs`)
- [ ] Extend Rush parser with array types, for-loops, let bindings
- [ ] Implement scalar Cranelift codegen from IR
- [ ] Working end-to-end: shell script → scalar JIT execution

### Phase 2: RISC-V Interpreter Core
- [ ] Create `crates/rv64emu/` crate (binary)
- [ ] RV64I base decoder + executor (50 instructions)
- [ ] RV64M (mul/div), RV64F/D (float/double)
- [ ] Flat 4KB-paged virtual address space
- [ ] Memory access tracer (per-page AccessLog + AccessSummary)
- [ ] Basic block execution counting

### Phase 3: Pattern Detection + SIMD Codegen
- [ ] IR-level pattern detector (MapLoop identification)
- [ ] Stride analysis from memory traces
- [ ] Cranelift SIMD vector codegen (i32x4, f64x2)
- [ ] RISC-V trace → IR lowering (RVV sequences → IR MapLoop)
- [ ] Scalar epilogue for remainder elements
- [ ] Benchmarks: scalar vs SIMD on 1M-element arrays

### Phase 4: Checkpoint & Cloud Migration
- [ ] VMCheckpoint struct + binary serialization
- [ ] Dirty page tracking (per-page DIRTY flag + dirty_mask)
- [ ] Full snapshot capture/restore
- [ ] REST API for instance management
- [ ] Migration protocol between nodes
- [ ] Incremental migration (iterative dirty-page sync)

### Phase 5: GPU Lowering
- [ ] wgpu compute shader compilation from IR
- [ ] Page → GPU buffer mapping (constant/input/output arrays)
- [ ] Coherency protocol (DIRTY/STALE/COHERENT state machine)
- [ ] Execution scheduler (size-based heuristics: CPU SIMD vs GPU)
- [ ] GPU fence + async dispatch pipeline

### Phase 6: Production Hardening
- [ ] RVV 1.0 full support (vector load/store, arithmetic, reductions)
- [ ] Multiple GPU backends (wgpu + optional CUDA)
- [ ] Live migration with sub-millisecond pause
- [ ] Multi-tenant cloud platform
- [ ] Tooling: `shardc` (compiler), `sharddbg` (debugger), `shardtop` (profiler)

## Crate Map

```
crates/
  shardsimd/          Shell DSL + shared IR + multi-backend codegen
  rv64emu/            RISC-V interpreter + tracer
  shared/             (future) common types if duplication emerges
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Cranelift for CPU codegen | Native Rust, simple build, fast JIT, auto-SIMD lowering |
| wgpu for GPU | Cross-platform (Vulkan/Metal/DX12), no LLVM needed, WGSL is standard |
| Custom mid-IR | Cranelift CLIF is too low-level for pattern detection |
| 4KB pages | Matches standard page size, fine-grained dirty tracking |
| Flat page table | Simpler than hierarchical; 64-bit VAS with sparse allocation |
| Checkpoint binary format | Not JSON/Protobuf — 4KB page data bulk makes binary encoding 10x faster |
| Rush parser reuse | Already has working lexer/walker/expand pipeline |
