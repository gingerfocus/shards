# parser-sh

The shell lexer, token walker, and command parser used by `shards` in `sh`
mode. Parsed words retain variable and command substitutions as syntax. The
`shards` executor expands them when each command runs and owns all shell state.

The library exposes `lexer::Lexer`, `walker::Walker`, and `command::Parser`.
