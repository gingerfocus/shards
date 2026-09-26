# parser-rust

Parses one Rushi external command call into a `Call` with a program and string
arguments. Use `parse_call("ls(\"-la\")")`. Parsing does not execute a process;
the `shards` crate handles execution.
