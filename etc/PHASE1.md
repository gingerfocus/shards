# Phase 1: Shared IR + Scalar JIT (Shell DSL)

**Goal:** End-to-end working pipeline from shell-script syntax to scalar
Cranelift JIT execution. Establishes all shared types and interfaces.

## Crate: `crates/shardsimd/`

### Week 1: Project Scaffold

- [ ] `cargo init --lib crates/shardsimd` (binary crate)
- [ ] Add workspace member to root `Cargo.toml`
- [ ] Dependencies:
  - [ ] `cranelift-codegen = "0.112"`
  - [ ] `cranelift-jit = "0.112"`
  - [ ] `cranelift-module = "0.112"`
  - [ ] `cranelift-native = "0.112"`
  - [ ] `rush-core` (workspace dep from `parsers/rush/rush-core`)
  - [ ] `clap = "4"` (CLI args)
  - [ ] `error-stack` (from workspace `resu`)
  - [ ] `log` (workspace dep)
- [ ] CI: `cargo build` passes with all deps

### Week 1-2: IR Definition (`src/ir.rs`)

- [ ] `ScalarType` enum: `I8, I16, I32, I64, F32, F64`
- [ ] `Local` (SSA variable, index into local table)
- [ ] `Block` (basic block label, index into block table)
- [ ] `Const` (immediate value: int or float)
- [ ] `ScalarOp` enum:
  - [ ] `Add(lhs, rhs)`, `Sub`, `Mul`, `Div`
  - [ ] `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`
  - [ ] `And`, `Or`, `Xor`, `Not`
  - [ ] `Load(addr)`, `Store(addr, value)`
  - [ ] `Call(name, args)`
  - [ ] `Branch(cond, then_block, else_block)`
  - [ ] `Jump(block)`
  - [ ] `Return(value)`
- [ ] `ArrayLit { ty: ScalarType, data: Vec<Const> }`
- [ ] `MapLoop { iterator: Local, start: Local, end: Local, body: Vec<ScalarOp>, output_array: Local, input_arrays: Vec<Local> }`
- [ ] `Function { name: String, params: Vec<(String, ScalarType)>, return_ty: ScalarType, body: Vec<Block> }`
- [ ] `Module { functions: Vec<Function>, globals: HashMap<String, ArrayLit> }`
- [ ] Display/Debug impls for all types
- [ ] Validation pass: verify SSA well-formedness, block terminators, type consistency

### Week 2: Parser Extension (`src/parse.rs`)

- [ ] Reuse `rush_core::lexer::Lexer` for tokenization
- [ ] Reuse `rush_core::walker::Walker` for token→TreeItem
- [ ] Extend `Token` types (in this crate, not upstream):
  - [ ] `keyword::Let`, `keyword::For`, `keyword::In`, `keyword::Fn`
  - [ ] `TypeAnnotation`: `@i8`, `@i16`, `@i32`, `@i64`, `@f32`, `@f64`
  - [ ] `RangeLiteral`: `0..100`
  - [ ] `ArrayBracket`: `[`, `]`
  - [ ] `Comma`: `,`
  - [ ] Math operators recognized: `+`, `-`, `*`, `/`
- [ ] Parser functions:
  - [ ] `parse_let()`: `let @i32 x = expr`
  - [ ] `parse_for()`: `for i in start..end { body }`
  - [ ] `parse_array_lit()`: `[1, 2, 3, 4]`
  - [ ] `parse_expr()`: arithmetic, variables, array indexing `a[i]`
  - [ ] `parse_statement()` → dispatches to above
- [ ] Lower to IR:
  - [ ] `Let` → IR `Local` + assignment
  - [ ] `For` range loop with array body → IR `MapLoop` if body qualifies
  - [ ] `For` range loop with side effects → IR scalar loop blocks
  - [ ] Arithmetic → IR `ScalarOp`
- [ ] Tests:
  - [ ] Parse `let x = 42` → correct IR
  - [ ] Parse `for i in 0..8 { c[i] = a[i] + b[i] }` → IR MapLoop
  - [ ] Parse `for i in 0..8 { print(a[i]) }` → IR scalar loop
  - [ ] Parse error: mismatched types
  - [ ] Parse error: unclosed bracket

### Week 3: Scalar Cranelift Codegen (`src/codegen/scalar.rs`)

- [ ] Port working code from `shards/src/jit.rs` (lines 1-462)
- [ ] Adapters:
  - [ ] IR `ScalarType` → Cranelift `types::Type`
  - [ ] IR `Const` → Cranelift `iconst`/`f64const`
  - [ ] IR `ScalarOp` → Cranelift instructions
- [ ] Codegen for:
  - [ ] `Add/Sub/Mul/Div` → `iadd`/`isub`/`imul`/`udiv` (int) or `fadd`/`fsub`/`fmul`/`fdiv` (float)
  - [ ] Comparisons → `icmp`/`fcmp`
  - [ ] `Branch` → `brif` with then/else blocks + merge block
  - [ ] `Jump` → `jump`
  - [ ] `Return` → `return_`
  - [ ] `Call` → external function calls via `Linkage::Import`
  - [ ] `Load`/`Store` → `load`/`store` with heap base
- [ ] Function compilation:
  - [ ] Declare params as Cranelift function params
  - [ ] Build CFG from IR blocks
  - [ ] Finalize, compile, get function pointer
- [ ] Tests:
  - [ ] Compile and execute `fn() -> i64 { return 42 }` → verify returns 42
  - [ ] Compile and execute `fn(a: i64, b: i64) -> i64 { return a + b }` → verify result
  - [ ] Compile and execute `if/else` → verify branching
  - [ ] Compile scalar loop: sum 1..100

### Week 4: Runtime + Integration (`src/runtime.rs`, `src/main.rs`)

- [ ] Heap: `Vec<u8>` with typed array views
  - [ ] `allocate_typed(ty: ScalarType, count: u64) -> (base_addr, size)`
  - [ ] `read_scalar(addr, ty) -> Const`
  - [ ] `write_scalar(addr, ty, value)`
- [ ] Built-in functions:
  - [ ] `print_i64`, `print_f64` (call libc printf)
  - [ ] `read_i64`, `read_f64` (call libc scanf)
  - [ ] `alloc_array(ty, count)`, `free_array(addr)`
- [ ] Module execution:
  - [ ] Compile all functions
  - [ ] Call entry function
  - [ ] Report execution time (wall clock)
- [ ] CLI:
  - [ ] `shardsimd run <file.sd>` — load DSL, compile, execute
  - [ ] `shardsimd compile <file.sd>` — parse and print IR
  - [ ] `shardsimd --emit=ir <file.sd>` — print IR
  - [ ] `shardsimd --emit=clif <file.sd>` — print Cranelift IR
  - [ ] `shardsimd --bench <file.sd>` — run N times, report avg
- [ ] Test script: `scripts/bench_vec_add.sd` — 1M element vector add, scalar path

### Week 4: Docs
- [ ] README.md for `crates/shardsimd/`
- [ ] Example programs in `examples/`:
  - [ ] `hello.sd` — print a value
  - [ ] `math.sd` — arithmetic and conditionals
  - [ ] `loop_sum.sd` — scalar sum loop
  - [ ] `vec_add.sd` — element-wise vector add (scalar path)
