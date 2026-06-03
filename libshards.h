// #include <stdarg.h>
#include <stdint.h>
// #include <stdlib.h>
#include <stdbool.h>

struct ShardsValue;

struct FfiString {
  uint8_t *ptr;
  uintptr_t len;
  uintptr_t cap;
};

struct ShardsIdentifier {
  enum IdTag {
    Variable,
    Literal,
  };

  struct Variable_Body {
    struct FfiString name;
  };

  struct Literal_Body {
    // struct ShardsValue val;
  };

  enum IdTag tag;
  union {
    struct Variable_Body variable;
    struct Literal_Body literal;
  };
};

struct ShardsOperation {
  enum OpTag {
    ScriptCall,
    Add,
    Subtract,
    Multiply,
  };

  struct ScriptCall_Body {
    struct FfiString _0;
  };

  enum OpTag tag;
  union {
    struct ScriptCall_Body script_call;
  };
};

struct ShardsToken {
  enum TkTag {
    Identifier,
    Operation,
  };

  struct Identifier_Body {
    struct ShardsIdentifier _0;
  };

  struct Operation_Body {
    struct ShardsOperation _0;
  };

  enum TkTag tag;
  union {
    struct Identifier_Body identifier;
    struct Operation_Body operation;
  };
};

struct ShardsAst {
  /// A flag that repersents if the current Ast is valid. When true treated
  /// as if the rest of None was returned. With the exceptions that the data
  /// returned must still be valid so that it can be properly freed.
  bool is_valid;
  /// A pointer to the first token in a collection of tokens that make up
  /// the AST.
  struct ShardsToken *tokens_pointer;
  /// The number of tokens that come after the pointer, used when converting
  /// it to a rust type.
  uintptr_t tokens_count;
};

/// Used to construct an invalid Ast from external code.
///
/// # Safety
/// If you are using Rust use [`ShardsAst::invalid`] instead.
struct ShardsAst shards_invalid_ast();
