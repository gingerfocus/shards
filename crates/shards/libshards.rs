//! The idomatic rust abstraction over the raw ast
//!
//! Type for working with the shards framework. The way it strange serialization
//! pattern. Your end goal is to provide an ShardsAst as a return to your cdylib.
//! this can be done in what ever way you want but this library is provided as
//! a way to make the easier.
//!
//! When using rust it is recomended to construct it using the Ast type an
//! then convert it to the other. Other languages can either use the interface
//! directly or to use a package that abstracts this interface.
#![deny(
    // missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unused_imports,
    dead_code,
    unused_crate_dependencies
)]
// #![feature(vec_into_raw_parts)]

pub type ParseFuncSig = fn(*const u8, usize) -> libshards_sys::ShardsAst;

/// Marker Trait
pub trait ParseFunction {
    fn parse(&self, data: &str) -> Option<Ast>;
}

impl ParseFunction for ParseFuncSig {
    fn parse(&self, data: &str) -> Option<Ast> {
        return Some(Ast {
            tokens: vec![
                Token::Operation(Operation::ScriptCall {
                    name: "echo".to_owned(),
                    args: vec!["hello".to_owned(), "world".to_owned()],
                }),
                Token::Operation(Operation::ScriptCall {
                    name: "ls".to_owned(),
                    args: vec!["-a".to_owned()],
                }),
            ],
        });
        // let ast = self(data.as_ptr(), data.len());
        // if ast.state == libshards_sys::ShardsAstState_Errors {
        //     let err = ast.infos.error;
        //     match err {
        //         libshards_sys::ShardsParseError_Invalid => {},
        //         libshards_sys::ShardsParseError_BadToken => {},
        //         _ => {}
        //     }
        //     return None;
        // } else {
        //     let data = ast.infos.datas;
        //     data.ptr
        //
        // }
        // .map_err(|e| log::error!("{e:?}"))
        return None;
    }
}

// use libshards_sys::ParseError;
pub use libshards_sys::{
    ShardsAst,
    // ShardsIdentifier, ShardsOperation, ShardsToken, ShardsType, ShardsValue,
};

// TODO: find all these missing impls
//
// impl TryFrom<libshards_core::ShardsAst> for Ast {
//     type Error = ();
//
//     fn try_from(value: libshards_core::ShardsAst) -> Result<Self, Self::Error> {
//         todo!()
//     }
// }

#[derive(Debug)]
pub struct Ast {
    /// A pointer to the first token in a collection of tokens that make up
    /// the AST.
    pub tokens: Vec<Token>,
}

impl From<Ast> for libshards_sys::ShardsAst {
    /// Turns a valid ast into a an ast that can pass properly pass through
    /// library boundieres.
    ///
    /// # Safety
    /// Leaks the tokens of this ast. The person who recives the ast after this
    /// is responsible for freeing the memory.
    fn from(value: Ast) -> Self {
        todo!();

        // let data = value
        //     .tokens
        //     .iter()
        //     .cloned()
        //     .map(Into::<libshards_sys::ShardsToken>::into)
        //     .collect::<Vec<libshards_sys::ShardsToken>>()
        //     .leak();
        //
        // libshards_core::ShardsAst {
        //     is_valid: true,
        //     tokens_count: data.len(),
        //     tokens_pointer: data.as_mut_ptr(),
        // }
    }
}

// impl From<Option<Ast>> for ShardsAst {
//     fn from(value: Option<Ast>) -> Self {
//         match value {
//             Some(a) => a.into(),
//             None => unsafe { libshards_core::shards_invalid_ast() },
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum Token {
    Identifier(Identifier),
    Operation(Operation),
}

// impl From<Token> for ShardsToken {
//     fn from(value: Token) -> Self {
//         match value {
//             Token::Identifier(ident) => Self::Identifier(ident.into()),
//             Token::Operation(op) => Self::Operation(op.into()),
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum Identifier {
    Variable { name: String },
    Literal { value: Value },
}

// impl From<Identifier> for ShardsIdentifier {
//     fn from(value: Identifier) -> Self {
//         match value {
//             Identifier::Variable { name } => ShardsIdentifier::Variable { name: name.into() },
//             Identifier::Literal { value } => ShardsIdentifier::Literal { val: value.into() },
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum Operation {
    /// A script to call passed as its raw parts
    ScriptCall {
        name: String,
        args: Vec<String>,
    },
    Add,
    Subtract,
    Multiply,
}

// impl From<Operation> for ShardsOperation {
//     fn from(value: Operation) -> Self {
//         match value {
//             Operation::ScriptCall { name } => Self::ScriptCall(name.into()),
//             Operation::Add => Self::Add,
//             Operation::Subtract => Self::Subtract,
//             Operation::Multiply => Self::Multiply,
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum Value {
    /// Useful for languages that are not strongly typed
    Untyped(Box<[u8]>),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    // String,
    // Array(Type),
}

// impl From<Value> for ShardsValue {
//     fn from(value: Value) -> Self {
//         match value {
//             Value::Untyped(data) => ShardsValue {
//                 variable_type: libshards_core::ShardsType::Untyped,
//                 data,
//             },
//             Value::U32(_) => todo!(),
//             Value::U64(_) => todo!(),
//             Value::I32(_) => todo!(),
//             Value::I64(_) => todo!(),
//         }
//     }
// }

// impl TryFrom<ShardsAst> for Ast {
//     type Error = ParseError;
//
//     fn try_from(value: ShardsAst) -> Result<Self, Self::Error> {
//         if !value.is_valid {
//             return Err(ParseError::NotValid);
//         }
//
//         let shards_tokens = unsafe {
//             Vec::from_raw_parts(value.tokens_pointer, value.tokens_count, value.tokens_count)
//         };
//
//         let in_len = shards_tokens.len();
//
//         let tokens: Vec<Token> = shards_tokens
//             .into_iter()
//             .flat_map(|shard| shard.try_into())
//             .collect();
//
//         let out_len = tokens.len();
//
//         if in_len > out_len {
//             Err(ParseError::BadTokens)
//         } else {
//             Ok(Ast { tokens })
//         }
//     }
// }

// impl TryFrom<ShardsToken> for Token {
//     type Error = String;
//
//     fn try_from(value: ShardsToken) -> Result<Self, Self::Error> {
//         match value {
//             ShardsToken::Identifier(ident) => match ident.try_into() {
//                 Ok(ident) => Ok(Token::Identifier(ident)),
//                 Err(e) => Err(e),
//             },
//             ShardsToken::Operation(op) => match op.try_into() {
//                 Ok(op) => Ok(Token::Operation(op)),
//                 Err(e) => Err(e),
//             },
//         }
//     }
// }

// impl TryFrom<ShardsIdentifier> for Identifier {
//     type Error = String;
//
//     fn try_from(value: ShardsIdentifier) -> Result<Self, Self::Error> {
//         match value {
//             ShardsIdentifier::Variable { name } => {
//                 let name = name.try_into().unwrap();
//                 Ok(Self::Variable { name })
//             }
//             ShardsIdentifier::Literal { val } => {
//                 val.try_into().map(|value| Self::Literal { value })
//             }
//         }
//     }
// }

// impl TryFrom<ShardsOperation> for Operation {
//     type Error = String;
//
//     fn try_from(value: ShardsOperation) -> Result<Self, Self::Error> {
//         match value {
//             ShardsOperation::ScriptCall(s) => {
//                 // TODO: check validity of passed values
//                 let name = String::try_from(s).unwrap();
//                 Ok(Self::ScriptCall { name })
//             }
//             _ => todo!(),
//         }
//     }
// }

// impl TryFrom<ShardsValue> for Value {
//     type Error = String;
//
//     fn try_from(value: ShardsValue) -> Result<Self, Self::Error> {
//         match value.variable_type {
//             ShardsType::Untyped => Ok(Value::Untyped(value.data)),
//             ShardsType::U32 => todo!(),
//             ShardsType::U64 => todo!(),
//             ShardsType::I32 => todo!(),
//             ShardsType::I64 => todo!(),
//         }
//     }
// }
