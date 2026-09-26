# Language execution levels

Each language has ten numbered source files. The files add syntax in roughly
increasing complexity. `sh`, `rust`, and `julia` files run through the `shards`
executable from a fresh temporary working directory. The runner checks exit
status and exact stdout. Levels beyond the current passing baseline are
probes: they appear in the report but do not fail the test.

Run the full report with:

```sh
cargo test -p shardscli --test language_levels -- --nocapture
```

Current execution baseline:

| Language | Required passing levels | Next probe |
| --- | --- | --- |
| Shell | 01–08 | 09: `for` loop |
| Rust-style | 01–06 | 07: nested call |
| Julia-style | 01–06 | 07: conditional |

Julia files run through the same executable as the other languages. The Julia
runtime implements output calls, variables, nested `string(...)`, language
switching, exit, and external calls. Control flow remains a later probe.

The shell levels cover a basic external command, quotes, variables, `||`,
`&&`, a pipeline, redirection, command substitution, a loop, and a function.
The Rust-style levels cover calls, arguments, multiple statements, language
switching, nested calls, variables, conditionals, and functions. Julia levels
progress from calls and arguments to variables, nesting, control flow,
functions, and a module.
