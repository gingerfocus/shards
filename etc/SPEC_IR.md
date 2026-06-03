# SPEC: Shared Intermediate Representation (IR)

## Overview

The shared IR is a mid-level SSA-based representation that sits between the
parser (DSL or RISC-V tracer) and the codegen backends (Cranelift scalar,
Cranelift SIMD, wgpu GPU). It is designed to be simple enough for pattern
detection yet expressive enough to lower to all three backends.

The IR is function-oriented: a `Module` contains named `Function`s, each
with a body of `Block`s.

## Design Principles

1. **SSA form**: All values are `Local`s (virtual registers). No mutable
   variables after assignment — SSA construction happens during IR emission.

2. **Structured control flow**: `Branch` (if/else) and `Jump` (goto) are
   preserved. Loops are either `MapLoop` (SIMD/GPU-candidate) or general
   scalar loops with backedge `Jump`s.

3. **Explicit memory**: `Load` and `Store` operations carry the base address
   and offset. Array indexing is explicit.

4. **Backend-agnostic**: No Cranelift or wgpu types leak into the IR.

## Types

```rust
struct Module {
    name:     String,
    functions: Vec<Function>,
    globals:  Vec<Global>,
}

struct Global {
    name:  String,
    kind:  GlobalKind,
    ty:    ScalarType,
    data:  Vec<u8>,
}

enum GlobalKind {
    Const,       // read-only, can go to GPU uniform buffer
    Mutable,     // read-write, can go to GPU storage buffer
}

enum ScalarType {
    I8, I16, I32, I64,
    F32, F64,
    Bool,
}

struct Function {
    name:      String,
    params:    Vec<(String, ScalarType)>,
    return_ty: Option<ScalarType>,      // None = void
    body:      Vec<Block>,
    locals:    Vec<LocalDecl>,          // all SSA variables
    blocks:    Vec<BlockId>,            // block index table
}

struct LocalDecl {
    name: String,
    ty:   ScalarType,
}

struct Block {
    label:   String,       // for debugging
    params:  Vec<Local>,   // block parameters (for merge blocks)
    ops:     Vec<IrOp>,
    terminator: Terminator,
}
```

## Operations

```rust
enum IrOp {
    /// Binary arithmetic: result = lhs OP rhs
    Add      { result: Local, lhs: Local, rhs: Local },
    Sub      { result: Local, lhs: Local, rhs: Local },
    Mul      { result: Local, lhs: Local, rhs: Local },
    Div      { result: Local, lhs: Local, rhs: Local },
    Rem      { result: Local, lhs: Local, rhs: Local },

    /// Bitwise
    And      { result: Local, lhs: Local, rhs: Local },
    Or       { result: Local, lhs: Local, rhs: Local },
    Xor      { result: Local, lhs: Local, rhs: Local },
    Shl      { result: Local, lhs: Local, rhs: Local },
    Shr      { result: Local, lhs: Local, rhs: Local },

    /// Unary
    Neg      { result: Local, src: Local },
    Not      { result: Local, src: Local },

    /// Comparisons (produce Bool)
    Eq       { result: Local, lhs: Local, rhs: Local },
    Ne       { result: Local, lhs: Local, rhs: Local },
    Lt       { result: Local, lhs: Local, rhs: Local },
    Le       { result: Local, lhs: Local, rhs: Local },
    Gt       { result: Local, lhs: Local, rhs: Local },
    Ge       { result: Local, lhs: Local, rhs: Local },

    /// Memory
    Load     { result: Local, base: Local, offset: Local, ty: ScalarType },
    Store    { base: Local, offset: Local, value: Local, ty: ScalarType },

    /// Constants
    ConstInt { result: Local, value: i64 },
    ConstFlt { result: Local, value: f64 },
    ConstBool{ result: Local, value: bool },

    /// Array element indexing: result = base + index * elem_size
    ArrayIndex { result: Local, base: Local, index: Local, elem_size: u32 },

    /// Call external function
    Call     { result: Option<Local>, name: String, args: Vec<Local> },

    /// Extract block parameter from predecessor
    GetParam { result: Local, param_idx: u32 },

    /// Copy (for SSA: used when a value is needed across blocks)
    Copy     { result: Local, src: Local },
}

enum Terminator {
    /// Conditional branch: if cond jump to true_block(args) else false_block(args)
    Branch {
        cond:        Local,
        true_block:  BlockId,
        true_args:   Vec<Local>,
        false_block: BlockId,
        false_args:  Vec<Local>,
    },

    /// Unconditional jump to block(args)
    Jump {
        target: BlockId,
        args:   Vec<Local>,
    },

    /// Return value (or void if None)
    Return {
        value: Option<Local>,
    },
}
```

## MapLoop (SIMD/GPU-Candidate Loop)

A `MapLoop` is a special function-level construct that represents an
element-wise loop over arrays:

```rust
struct MapLoop {
    /// The loop counter local
    iterator:       Local,

    /// Start and end values (inclusive or exclusive per kind)
    start:          Local,
    end:            Local,

    /// How the iteration proceeds
    kind:           LoopKind,

    /// The element-wise operations in the loop body.
    /// These must be pure (no side effects, no external calls, no control flow).
    body:           Vec<IrOp>,

    /// Array being written (one per MapLoop)
    output_array:   Local,
    /// Arrays being read
    input_arrays:   Vec<Local>,

    /// Element type of the arrays
    element_type:   ScalarType,
}

enum LoopKind {
    /// for i = start; i < end; i += 1 (exclusive end)
    RangeExclusive,
    /// for i = start; i <= end; i += 1 (inclusive end)
    RangeInclusive,
    /// for i = 0; i < array_len; i += 1
    FullArray,
}
```

The body of a MapLoop is restricted:
- Only `Add`, `Sub`, `Mul`, `Div`, `Neg`, `Abs`, `Sqrt`, `Min`, `Max`,
  `Clamp`, `Load`, `Store`, and `ArrayIndex` ops
- No `Call` (no side effects)
- No `Branch` (no control flow divergence)
- All loads are from `input_arrays`, the single store is to `output_array`

## IR Construction Example

DSL source:
```
let @f64 a = [1.0, 2.0, 3.0, 4.0]
let @f64 b = [5.0, 6.0, 7.0, 8.0]
let @f64 c = 0.0
for i in 0..4 {
    c[i] = a[i] + b[i] * 2.0
}
```

Resulting IR (simplified, with SSA locals):
```
Module "vec_add":
  Globals:
    a: Const, F64, [1.0, 2.0, 3.0, 4.0]
    b: Const, F64, [5.0, 6.0, 7.0, 8.0]

  Function "main" -> void:
    Locals:
      0: "a_base"    I64     (address of global a)
      1: "b_base"    I64     (address of global b)
      2: "c_base"    I64     (allocated output)
      3: "two"       F64     (constant 2.0)
      4: "i"         I64     (loop counter)
      5: "a_idx"     I64     (a[i] address)
      6: "b_idx"     I64     (b[i] address)
      7: "c_idx"     I64     (c[i] address)
      8: "a_val"     F64     (loaded a[i])
      9: "b_val"     F64     (loaded b[i])
     10: "b_mul"     F64     (b[i] * 2.0)
     11: "result"    F64     (a[i] + b[i] * 2.0)

    MapLoop:
      iterator: 4, start: 0, end: 4, kind: RangeExclusive
      element_type: F64
      output_array: 2 (c)
      input_arrays: [0, 1] (a, b)
      body:
        ConstFlt { result: 3, value: 2.0 }
        ArrayIndex { result: 5, base: 0, index: 4, elem_size: 8 }
        ArrayIndex { result: 6, base: 1, index: 4, elem_size: 8 }
        ArrayIndex { result: 7, base: 2, index: 4, elem_size: 8 }
        Load { result: 8, base: 5, offset: 0, ty: F64 }
        Load { result: 9, base: 6, offset: 0, ty: F64 }
        Mul { result: 10, lhs: 9, rhs: 3 }
        Add { result: 11, lhs: 8, rhs: 10 }
        Store { base: 7, offset: 0, value: 11, ty: F64 }
```

## Validation Pass

The IR module should pass validation before codegen:

```rust
fn validate(module: &Module) -> Result<(), Vec<IrError>> {
    let mut errors = Vec::new();

    for func in &module.functions {
        // 1. SSA: each Local is defined exactly once
        // 2. Types: all operations type-check
        // 3. Block terminators: every block has exactly one terminator
        // 4. Block reachability: all blocks reachable from entry
        // 5. Block parameters: passed args match parameter count
        // 6. MapLoop purity: body contains only allowed ops
        // 7. Variable scope: Locals defined before use
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

## RISC-V Trace → IR Lowering

When lowering from RISC-V execution traces (rather than DSL source), the
process is different — we reconstruct IR from observed instruction traces:

```rust
fn trace_to_ir(traces: &[InsnTrace], hot_pcs: &HashSet<u64>) -> Module {
    let mut module = Module::new("rv_trace");

    // 1. Identify loop headers from backedge traces
    let loops = identify_loops(traces);

    // 2. For each hot loop:
    for loop_info in &loops {
        let body_traces = extract_body(traces, loop_info);

        // 3. Map RISC-V registers to IR Locals
        let mut reg_map = HashMap::new();
        for trace in &body_traces {
            let ir_op = riscv_to_ir_op(trace, &mut reg_map);
            // ...
        }

        // 4. Classify as MapLoop if:
        //    - All loads are contiguous stride-1 from same base
        //    - All stores are contiguous stride-1 to same base
        //    - No side-effect instructions in body
        if is_map_loop(&body_traces) {
            module.add_map_loop(build_map_loop(&body_traces, &reg_map));
        } else {
            module.add_scalar_loop(build_scalar_loop(&body_traces, &reg_map));
        }
    }

    module
}

fn riscv_to_ir_op(trace: &InsnTrace, reg_map: &mut HashMap<u8, Local>) -> IrOp {
    let rd = reg(trace.rd);
    let rs1 = reg(trace.rs1);
    let rs2 = reg(trace.rs2);

    match trace.opcode & 0x7F {
        0x33 => match funct3(trace.opcode) {
            0x0 => IrOp::Add { result: rd, lhs: rs1, rhs: rs2 },  // add/sub
            0x1 => IrOp::Shl { result: rd, lhs: rs1, rhs: rs2 },
            // ... etc
        },
        0x03 => match funct3(trace.opcode) {
            0x3 => IrOp::Load { result: rd, base: rs1, offset: imm12(trace.opcode), ty: I64 }, // ld
            // ...
        },
        // ... etc
    }
}
```

## Display Format (Debug)

```
Function "vec_add":
  %0 = const.i64 0                 // index base
  %1 = const.i64 4                 // loop bound
  %2 = const.f64 2.0               // scalar constant
  map_loop %i in [%0 : %1] {
    %3 = array_index a_base, %i    // &a[i]
    %4 = array_index b_base, %i    // &b[i]
    %5 = load.f64 %3               // a[i]
    %6 = load.f64 %4               // b[i]
    %7 = mul.f64 %6, %2            // b[i] * 2.0
    %8 = add.f64 %5, %7            // a[i] + b[i] * 2.0
    store.f64 %8, c_base, %i       // c[i] = result
  }
```
