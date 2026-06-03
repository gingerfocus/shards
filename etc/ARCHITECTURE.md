# ShardVM Architecture

## High-Level Data Flow

```
                   Shell DSL                     RISC-V Binary
                  (let, for, @i32)               (.elf / .bin)
                        │                             │
                        ▼                             ▼
                  ┌──────────────┐           ┌─────────────────┐
                  │ Rush Lexer   │           │ RV64G Decoder   │
                  │ → Walker     │           │ → Instruction   │
                  └──────┬───────┘           └────────┬────────┘
                         │                            │
                         ▼                            ▼
                  ┌──────────────┐           ┌─────────────────┐
                  │ DSL Parser   │           │ RISC-V Interp.  │
                  │ → shardsimd  │           │ + Memory Tracer │
                  │   IR Module  │           │ + Page Table    │
                  └──────┬───────┘           └────────┬────────┘
                         │                            │
                         │              ┌─────────────┘
                         ▼              ▼
                  ┌──────────────────────────────┐
                  │     Pattern Detector          │
                  │  - MapLoop identification     │
                  │  - Stride analysis            │
                  │  - Access pattern classif.    │
                  └──────────────┬───────────────┘
                                 │
                    ┌────────────┼────────────┐
                    ▼            ▼            ▼
              ┌──────────┐ ┌──────────┐ ┌──────────┐
              │ Scalar   │ │ SIMD     │ │ GPU      │
              │ Cranelift│ │ Cranelift│ │ wgpu     │
              │ Codegen  │ │ Codegen  │ │ Codegen  │
              └──────────┘ └──────────┘ └──────────┘
                                 │
                                 ▼
              ┌──────────────────────────────────┐
              │       Execution Scheduler         │
              │  Threshold-based tier decisions   │
              │  Coherency management             │
              └──────────────────────────────────┘
                                 │
                                 ▼
              ┌──────────────────────────────────┐
              │       Cloud Platform              │
              │  Checkpoint capture/restore       │
              │  REST API for migration           │
              │  Incremental dirty-page sync      │
              └──────────────────────────────────┘
```

## Crate Architecture

```
crates/
├── shardsimd/                    # Binary: shell DSL + shared IR + codegen
│   ├── Cargo.toml                # deps: rush-core, cranelift-*, wgpu
│   └── src/
│       ├── main.rs               # CLI entry: run, compile, bench
│       ├── parse.rs              # Shell DSL parser (extends Rush)
│       ├── ir.rs                 # Shared mid-level IR (Module, Function, etc.)
│       ├── pattern.rs            # Pattern detector (MapLoop, strides)
│       ├── scheduler.rs          # Execution scheduler (tier selection)
│       └── codegen/
│           ├── mod.rs            # Unified codegen interface
│           ├── scalar.rs         # Cranelift scalar JIT
│           ├── simd.rs           # Cranelift SIMD (vector) JIT
│           └── gpu.rs            # wgpu compute shader compilation
│
├── rv64emu/                      # Binary: RISC-V interpreter + tracer
│   ├── Cargo.toml                # deps: libc (for mmap), clap
│   └── src/
│       ├── main.rs               # CLI entry: run, trace, checkpoint
│       ├── decode.rs             # Instruction decoder (RV64G + RVV)
│       ├── execute.rs            # Instruction execution
│       ├── regfile.rs            # Register state (GPRs, FPRs, CSRs)
│       ├── memory.rs             # Flat memory model, page table, VAS
│       ├── tracer.rs             # Memory access logging + summarization
│       ├── rv_pattern.rs         # RISC-V trace → shared IR lowering
│       ├── assembler.rs          # ELF / .bin file loader
│       ├── checkpoint.rs         # VMCheckpoint serialize/deserialize
│       └── cloud.rs              # REST API client for cloud integration
│
└── shared/                       # (future) Common types
    └── src/
        └── lib.rs                # IR types, PageFlags, Checkpoint format
```

## Component Interaction Diagram

```
                  Interpretation                   Detection
 ┌────────┐     ┌────────────────┐     ┌─────────────────────────┐
 │ Source │────▶│ RV64EMU        │────▶│ PATTERN DETECTOR        │
 │ Input  │     │                │     │                         │
 └────────┘     │ decode.rs      │     │ Per-page:               │
                │ execute.rs     │     │   AccessLog → Summary   │
                │ regfile.rs     │     │   stride_histogram      │
                │ memory.rs      │     │   access_pattern        │
                │ tracer.rs      │     │                         │
                └───────┬────────┘     │ Per-basic-block:        │
                        │              │   (PC, addr, size)      │
                        │              │   stream                │
                        ▼              │                         │
                ┌────────────────┐     │ Loop detection:         │
                │ PAGE TABLE     │     │   backedge analysis     │
                │                │     │   MapLoop identification│
                │ HashMap<PFN,   │     │   scalar→element-wise   │
                │   PageDesc>    │     │   verification          │
                │                │     └───────────┬─────────────┘
                │ flags:         │                 │
                │  R|W|X|DIRTY   │                 ▼
                │  GPU_R|GPU_W   │     ┌─────────────────────────┐
                │  COHERENT|STALE│     │ SCHEDULER               │
                │                │     │                         │
                │ access_log[]   │     │ if count < THRESH:      │
                │ dirty_mask[64] │     │   continue interpreting  │
                └────────────────┘     │                         │
                                       │ if loop + stride1:      │
                                       │   → lower_to_simd()     │
                                       │ if loop + large data:    │
                                       │   → lower_to_gpu()      │
                                       │                         │
                                       │ Manage coherency:       │
                                       │   DIRTY→upload to GPU   │
                                       │   STALE→download from   │
                                       └─────────────────────────┘
```

## Execution Tiers

| Tier | Backend | When Selected | Latency |
|------|---------|---------------|---------|
| T0: Interpreter | RV64EMU execute.rs | Cold code (< 100 executions) | ~1us/insn |
| T1: Scalar JIT | Cranelift scalar | Hot simple blocks, no arrays | ~100us compile + ns exec |
| T2: SIMD JIT | Cranelift vector | Hot MapLoop, stride-1, < 10K elements | ~500us compile + ns exec |
| T3: GPU compute | wgpu compute shader | Hot MapLoop, > 10K elements, or gather | ~50ms compile + us dispatch |

Tier transitions are one-way (up). Once lowered, code stays lowered.
On migration to a new node, lowered regions are rebuilt from `LoweredRegion` records.
