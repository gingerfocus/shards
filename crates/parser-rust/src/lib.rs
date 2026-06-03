//! A library for parsing and manipulating Rust source code
//! into a ast a general ast.

use libshards::{Ast, Identifier, Operation, ShardsAst, Token};

/// Main external entry point for the library, this is used by non-rust and
/// dynamic contexts.
///
/// # Safety
/// This function dereference the pointer to string it is given. You are
/// responsible for assuming that the value pased into here is never mutated
/// as is assumed to be dropped.
#[no_mangle]
pub unsafe extern "C" fn parse(c: *const u8, len: usize) -> ShardsAst {
    let Ok(s) = std::str::from_utf8(std::slice::from_raw_parts(c, len)) else {
        eprintln!("Failed to parse string");
        // return None.into();
        todo!()
    };

    parse_safe(s).into()
}

/// Main entry point to the function when working in safe-rust. This is for if
/// you decide to include the library naticly or somthing.
pub fn parse_safe(s: &str) -> Ast {
    dbg!(&s);

    // let syntax = syn::parse_file(&s).expect("Unable to parse file");
    // let Some(shebang) = syntax.shebang else {
    //     panic!("No shebang found");
    // };

    // Debug impl is available if Syn is built with "extra-traits" feature.
    // println!("{:#?}", syntax);

    // this needs to leak otherwise it is a segfault
    // Token::Identifier(Identifier::Literal { value: todo!()}),
    //

    // let parse = format!(
    //     "fn main() {{
    //     {s}
    // }}"
    // );

    // let syntax = syn::parse_file(&parse).expect("Unable to parse file");
    // let Some(shebang) = syntax.shebang else {
    //     panic!("No shebang found");
    // };

    // match rushi::ast_from_str(&parse) {
    //     Some(s) => Ok(Ast {
    //         args: s,
    //     }),
    //     None => Err(anyhow::anyhow!("Failed to parse ast").into()),
    // }

    // Ok(Ast {
    //     args: s.split_whitespace().collect::<VecDeque<&str>>(),
    // })

    // Ok(Self { args: syntax })

    const A: [u8; 4] = *b"test";

    let mut tokens = vec![
        Token::Operation(Operation::ScriptCall {
            name: String::from("ls"),
        }),
        Token::Identifier(Identifier::Literal {
            value: libshards::Value::Untyped(Box::new(A)),
        }), // Token::Operation(Operation::ScriptCall("ls".into())),
    ];

    tokens.shrink_to_fit();

    Ast { tokens }
}
