# Shards
Shards is an experimental shell with one executable and separate language
parsers. It currently runs Rust-style external calls, basic `sh` commands,
and Julia-style calls.

The active runtime lives in `crates/shards/`. Earlier builtin and environment
source is kept in [the archive](etc/archive/legacy-shards/README.md).

## Run Shards

Run `cargo run -- -c 'ls()'` from the repository root, or run `cargo run`
to start the interactive prompt in Rust mode. Use
`cargo run -- --lang sh -c 'ls -la'` for shell syntax. Rust-mode arguments can
be passed as strings or simple bare values, for example `ls("-la")` or
`ls(-la)`.

Use `cargo run -- --lang julia -c 'println("hello")'` for Julia-style calls,
or pass a `.jl` file after `--lang julia`. Julia mode supports `print`,
`println`, `string`, assignments, and direct external command calls.

## Switch languages

At the shell prompt, type `shards lang rust` or `shards lang julia` to switch
syntax. In Rust or Julia mode, use `shards(lang, sh|rust|julia)`. The prompt
shows the active language. Each language keeps its session state when switching.

## Parser crates

| Crate | Current scope |
| --- | --- |
| `parser-sh` | Shell lexer, token walker, and command parser. |
| `parser-rust` | Rushi function-style command calls such as `ls("-la")`. |
| `parser-julia` | Julia-style calls, assignments, and nested calls. |

`shards` is the executable frontend. Julia control flow and functions are not
yet supported.

## File-based language tests

Run `cargo test -p shardscli --test language_levels -- --nocapture` to execute
the [numbered language fixtures](tests/fixtures/README.md)
and see each level's result. Shell levels 01–08 currently pass, including the
basic command in level 01. Rust-style levels 01–06 and Julia levels 01–06
also execute successfully.
