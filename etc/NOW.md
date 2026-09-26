# TODO
## What Actually Works (Keep)
| Component | Status | Value |
| Rush lexer (rush-core/src/lexer.rs) | Works for basic tokens (idents, pipes, spaces, quotes) | The single best piece of code in the project. Clean iterator API, 1 test, handles the shell tokenization you'd need for the DSL parser. Only read_escape has a todo!() for odd escapes. |
| Rush walker (rush-core/src/walker.rs) | Works for Ident, Space, Newline, Pipe, &&, ||, quotes, vars, redirect | 15 todo!() for edge tokens (braces, globs, tabs). Covers 80% of what your DSL parser needs.
| Rush drive (rush/src/drive.rs) | Simple commands + pipes work via libc::pipe2 | Clean pipeline execution. &&/||/! are todo!(). Good reference for pipe management.
| Rush interactive prompt (rush/src/parse.rs) | Full readline with history, raw mode, Ctrl-C/D/L | This is genuinely polished.
| Rush StaticMap (util.rs) | Simple string→string map | Only bit of util.rs without UB.
| CLI args (crates/shardscli/src/cli.rs) | Fully functional clap parser | Minor typo in field name but works.
| Cranelift JIT (crates/shards/src/jit.rs lines 1-462) | Compilable toy-language JIT with control flow, function calls, globals | The compile() has todo!() for the parser call, but the FunctionTranslator and translate() are real. This is the template for your scalar codegen.
| shardslsp core (client.rs, transport.rs, jsonrpc.rs) | Full LSP client with initialize, completions, hover, goto-def, etc. | Legitimate piece of software. Only flaw: hardcoded /tmp/stardust workspace root.

## What Exists But Needs Heavy Rework
Component	Issue	Verdict
Rush &&/||/!	3x todo!() in drive.rs	Extend in place or replace
Rush subshell, redirection	Multiple todo!()	Not needed for your DSL parser initially
Rush util.rs	Duplicated verbatim in both rush-core/ and rush/ with different crate paths. AtomicBuffer is unsound (UB).	Delete the rush/src/util.rs copy, keep only rush-core/src/util.rs. Remove AtomicBuffer/AtomicSlice entirely.
expr.rs AST (crates/shards/src/ast.rs)	Clean enum but only 22 scalar types. No arrays, no vectors.	You're already designing your own IR. This is a reference, not reusable directly.
Rush shell binary	shell.rs:249 has a todo!() in run_with_output — every shell session exits via panic.	The shell is 80% there but the final todo!() makes it unusable for non-interactive piped execution. Fix is ~5 lines.

## What Must Be Rewritten Entirely
Component	Issue	Replacement
libshards crate	From<Ast> is todo!(), mock parser returns hardcoded data, 95% commented-out code referencing stale types that don't exist in libshards-sys	Delete entirely. Your new crates/shardsimd/src/ir.rs replaces it.
libshards-sys header	Only has ShardsAst (error envelope). No token types, no shards_parse() C function. Bindgen generates 3 structs total.	Not needed for standalone compiler path. If you eventually make a shard plugin, rewrite the header from scratch.
rushi / rust-shard / stardust	All are 100% stub. rushi returns hardcoded ls, rush-shard is todo!(), stardust prints "Hello World".	Not needed for your plan. Can delete or leave as-is.
OpCode/ByteCode (opcod.rs, bytes.rs)	Both are empty structs with dbg!() stubs. OpCode::reduce() does nothing.	Your IR replaces the entire pipeline.
exec.rs	1,229 lines, ~99% commented-out C++. Uses undefined Process type and C macro _PATH_BSHELL.	Delete. Rush's drive.rs is the working replacement.
config/ (mod.rs, abbrs.rs, line.rs)	All commented out of the build. Reference undefined types (UserState, SourceRange, WString). Fish-shell port attempt that never finished.	Delete.
pipes/ (mod.rs, streams.rs)	streams.rs is 3x todo!() stubs. fds.rs has open_cloexec/exec_close (real) but everything else commented out.	Keep fds.rs as reference, delete streams.rs.
