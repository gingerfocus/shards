# parser-julia

Julia-style statement parser. `parse_call("ls(\"-la\", path)")` returns a
`Call` whose arguments distinguish strings from identifiers. `parse_statement`
also accepts a simple assignment, and call arguments may contain nested calls.
The `shards` crate evaluates these statements in Julia mode. Control flow and
function definitions are not yet parsed.
