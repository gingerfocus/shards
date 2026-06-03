#include <stdlib.h>

struct ShardsSlice {
  void *ptr;
  size_t len;
};
enum ShardsParseError { Invalid, BadToken };

enum ShardsAstState { Errors, Tokens };

struct ShardsAst {
  enum ShardsAstState state;

  union {
    enum ShardsParseError error;
    struct ShardsSlice datas;
  } infos;
};

// #[repr(C)]
// #[derive(Debug)]
// pub enum ShardsToken {
//     Identifier(ShardsIdentifier),
//     Operation(ShardsOperation),
// }
//
// #[repr(C)]
// #[derive(Debug)]
// pub enum ShardsIdentifier {
//     Variable { name: FfiString },
//     Literal { val: ShardsValue },
// }
//
// #[repr(C)]
// #[derive(Debug)]
// pub enum ShardsOperation {
//     // Takes the raw parts of an owned String
//     ScriptCall(FfiString),
//     Add,
//     Subtract,
//     Multiply,
// }
//
// #[repr(C)]
// #[derive(Debug)]
// pubns struct ShardsValue {
//     /// A type hint for what the data is. Can use the None value to let
//     /// it be guessed
//     pub variable_type: ShardsType,
//     // A collection of bytes that make up the data
//     pub data: Box<[u8]>,
// }
//
// #[repr(C)]
// #[derive(Debug)]
// pub enum ShardsType {
//     /// Useful for languages that are not strongly typed
//     Untyped,
//     U32,
//     U64,
//     I32,
//     I64,
//     // String,
//     // Array(Type),
// }
//
// #[repr(C)]
// #[derive(Debug)]
// pub struct FfiString {
//     pub ptr: *mut u8,
//     pub len: usize,
//     pub cap: usize,
// }
//
// impl From<String> for FfiString {
//     fn from(value: String) -> Self {
//         let (ptr, len, cap) = value.into_raw_parts();
//         Self { ptr, len, cap }
//     }
// }
//
// impl TryFrom<FfiString> for String {
//     type Error = resu::Report<FfiStringError>;
//
//     fn try_from(value: FfiString) -> Result<Self, Self::Error> {
//         if value.ptr.is_null() {
//             return Err(resu::Report::new(FfiStringError::Null));
//         }
//
//         Ok(unsafe { String::from_raw_parts(value.ptr, value.len, value.cap) })
//     }
// }
//
// impl Drop for FfiString {
//     fn drop(&mut self) {
//         log::error!("FfiString [`Drop`]ed while value leaked. This is likely a memory leak.")
//     }
// }
//
// #[derive(Debug)]
// pub enum FfiStringError {
//     Null,
// }
// impl fmt::Display for FfiStringError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             FfiStringError::Null => f.write_str("pointer was null"),
//         }
//     }
// }
// impl resu::Context for FfiStringError {}

// } const ShardsAst = extern struct {
//   typed : ShardsAstType, datas : ShardsAstData

// const ShardsAstType = enum(usize){
//     errors,
//     tokens,
// };
//
// const ShardsAstTokenData = extern struct { data : *ShardsToken, size : usize,
// }; const ShardsAstData = extern union {
//   errors : ParseError, tokens : ShardsAstTokenData
// };
// const ShardsToken = struct {};
//
// const ParseError = enum(u8){Invalid, BadToken};

