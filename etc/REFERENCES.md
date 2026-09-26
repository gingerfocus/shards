# ShardVM Resource Guide

## RISC-V ISA

### Primary Sources
- [RISC-V Unprivileged ISA Specification](https://github.com/riscv/riscv-isa-manual/releases/latest)
  - Volume 1: Unprivileged (RV64I base, M, F, D, Q extensions)
  - Read: Chapters 1-5 (base integer), Chapter 7 (M), Chapters 11-12 (F/D)
- [RISC-V "V" Vector Extension 1.0](https://github.com/riscv/riscv-v-spec/releases/latest)
  - Read: Chapters 1-7 (data types, configuration, memory ops, arithmetic)
  - The full spec is 200+ pages. For MVP, only implement:
    - `vsetvli` (set vector length)
    - `vle.v` / `vse.v` (unit-stride load/store)
    - `vadd.vv` / `vsub.vv` / `vmul.vv` (element-wise arithmetic)
- [RISC-V Privileged Specification](https://github.com/riscv/riscv-isa-manual/releases/latest)
  - Volume 2: Privileged (Machine mode CSRs, traps, interrupts)
  - Needed for `mstatus`, `mepc`, `mcause`, `mtvec`

### Quick Reference
- [riscv-software-src/riscv-opcodes](https://github.com/riscv-software-src/riscv-opcodes) — machine-readable instruction encoding
- [riscv-software-src/riscv-tests](https://github.com/riscv-software-src/riscv-tests) — official ISA compliance tests
- `spike` (RISC-V ISA simulator) — run with `--log-commits` for instruction-level tracing

### RISC-V Emulators to Study
- [rvemu](https://github.com/d0iasm/rvemu) — Educational RISC-V emulator in Rust. Clean, simple code. Good reference for decoder design.
- [libriscv](https://github.com/fwsGonzo/libriscv) — C++ RISC-V userspace emulator. Trap-and-emulate approach. Production-quality.
- [riscv-software-src/riscv-isac](https://github.com/riscv-software-src/riscv-isac) — ISA coverage analysis tool for RVV.

## Cranelift

### Primary Sources
- [Cranelift IR Reference](https://cranelift.dev/docs/ir/) — All instruction types
- [Cranelift JIT Tutorial](https://cranelift.dev/docs/tutorial/) — Based on the toy language JIT (our `jit.rs` is a fork of this)
- [Cranelift Vector Operations](https://cranelift.dev/docs/ir/#vector-operations) — `isplit`, `iconcat`, `insertlane`, `extractlane`, `swizzle`, `shuffle`

### Repositories
- [bytecodealliance/wasmtime](https://github.com/bytecodealliance/wasmtime) — Contains Cranelift under `cranelift/`
- Cranelift examples in the repo: `cranelift/jit/examples/` and `cranelift/frontend/src/frontend.rs`

### Key Concepts to Understand
- Instruction Selection: `cranelift_native::builder()` auto-detects host ISA and SIMD capabilities
- FunctionBuilder: The safe API for building CLIF. Use this, not raw `DataFlowGraph`.
- JITModule: Manages executable memory and relocations. Functions and data objects are declared first, then defined.
- Block parameters: Cranelift uses block params instead of phi nodes for SSA merging.
- Vector lowering: CLIF `i32x4` → AVX2 `vpaddd` on x86, NEON `add.4s` on aarch64. Automatic.

## wgpu (GPU)

### Primary Sources
- [wgpu Tutorial](https://sotrh.github.io/learn-wgpu/) — Best resource. Read the "Compute" section.
- [wgpu API Docs](https://docs.rs/wgpu) — Official Rust API reference
- [WGSL Specification](https://www.w3.org/TR/WGSL/) — The shading language
- [WebGPU Compute Example](https://github.com/gfx-rs/wgpu/tree/trunk/examples/src/compute) — Matrix multiply compute shader

### Key Concepts
- Adapter + Device: Discover GPU, create logical device
- Buffer: GPU-side memory (`BufferUsages::STORAGE` for read-write, `UNIFORM` for read-only)
- Bind Group Layout → Bind Group: Map buffers to shader resources
- Compute Pipeline: Compile WGSL source, create pipeline
- Command Encoder → Compute Pass → Dispatch → Submit
- Buffer mapping: `buffer.slice(..).map_async()` to read results back

### wgpu Crate Setup
```toml
[dependencies]
wgpu = "22"
pollster = "0.3"   # block_on for wgpu async init
bytemuck = "1"     # safe casting for GPU buffer data
```

## Memory Access Pattern Analysis

### Foundational Papers
- "Optimizing Compilers for Modern Architectures" (Allen & Kennedy, 2001) — Classic textbook. Chapters on dependence analysis, loop transformations.
- "Polyhedral Compilation Foundations" (Bastoul, 2004) — Mathematical foundation for loop nest analysis.
- "Automatic SIMD Vectorization of SSA-based Control Flow Graphs" (Moll & Hack, 2011) — Whole-function vectorization via program restructuring.
- "Intel 64 and IA-32 Architectures Optimization Reference Manual" — Practical guide to memory access patterns that benefit from SIMD.

### Practical References
- LLVM's LoopAccessAnalysis — checks if a loop is vectorizable by analyzing memory dependencies
- Halide scheduling — domain-specific language for image/array processing with explicit tiling/unrolling
- DynamoRIO instrumentation — production-grade binary instrumentation with memory trace capability

## Checkpoint / Live Migration

### VM Checkpointing
- [CRIU (Checkpoint/Restore In Userspace)](https://criu.org) — Linux process checkpointing. Study their approach to dirty page tracking.
- QEMU live migration — The classic VM migration protocol (iterative dirty-page sync)
- [Firecracker](https://github.com/firecracker-microvm/firecracker) — Lightweight VMM by AWS. Clean code in Rust. Study their VM state serialization.

### Binary Serialization
- capnproto (alternative to protobuf, zero-copy, good for pages)
- postcard (minimal `#[no_std]` serde format for embedded)
- Custom binary: TLV encoding for variable-length fields, fixed-width for bulk page data

## Existing Shards Code to Reuse

| Source | What | Use For |
|--------|------|---------|
| `parsers/rush/rush-core/src/lexer.rs` | Character→Token stream | Shell DSL tokenizer |
| `parsers/rush/rush-core/src/walker.rs` | Token→TreeItem (structured tokens) | Shell DSL parser input |
| `parsers/rush/rush/src/parse.rs` | TreeItem→Cmd (parse shell syntax) | Reference for parser design |
| `parsers/rush/rush/src/drive.rs` | Spawn processes with pipes | Reference for OS integration |
| `crates/shards/src/jit.rs` | Cranelift JIT (lines 1-462) | Scalar codegen template |
| `crates/shards/src/ast.rs` | Toy language AST (Expr enum) | Reference for IR design |
| `crates/libshards-sys/shards.h` | C ABI AST interchange | Reference for FFI crossing |
| `crates/shards/src/pipes/fds.rs` | AutoCloseFd, pipe creation | Reference for FD management |
