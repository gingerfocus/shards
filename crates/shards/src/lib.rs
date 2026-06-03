// #![deny(
//     // missing_docs,
//     missing_debug_implementations,
//     rust_2018_idioms,
//     unused_imports,
//     dead_code,
//     unused_crate_dependencies
// )]
// #![feature(vec_into_raw_parts)]

// mod builtins;
// mod config;
// mod env;
// mod exec;
// mod pipes;

mod ast;
// mod jit;
mod parser;
mod prelude;
mod types;

use crate::prelude::*;

#[derive(Debug)]
pub enum ShardsError {
    Ast,
    Run,
}
impl fmt::Display for ShardsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShardsError::Ast => f.write_str("failed to get a failed ast"),
            ShardsError::Run => f.write_str("failed to run the commnand"),
        }
    }
}
impl Context for ShardsError {}

// const OPTIMIZATION_LEVEL: u8 = 3;
fn parse(inter: &Interpreter, input: String) -> Result<(), ShardsError> {
    log::info!("read line from stdin");

    let ast = inter
        .loader
        .parse(&input)
        .ok_or(Report::new(ShardsError::Ast))?;

    log::info!("Got some ast");

    // let mut j = jit::JIT::new();
    //
    // j.create_data("hello_string", "hello world!\0".as_bytes().to_vec())
    //     .unwrap();
    //
    // // Pass the string to the JIT, and it returns a raw pointer to machine code.
    // let code = j.compile(&input).unwrap();

    // Cast the raw pointer to a typed function pointer. This is unsafe, because
    // this is the critical point where you have to trust that the generated code
    // is safe to be called.

    // let code = unsafe { mem::transmute::<_, fn() -> ()>(code) };
    // code();

    // let mut optc = OpCode::from(ast);
    // for _ in 0..=OPTIMIZATION_LEVEL {
    //     optc.reduce();
    // }
    // let bytes = ByteCode::from(optc);
    //
    // inter.eval(bytes).change_context(ShardsError::Run)?;

    Ok(())
}

// fn test() {
//     use tree_sitter::{Language, Parser};
//
//     let mut parser = Parser::new();
//
//     parser
//         .set_language(tree_sitter_rust::language())
//         .expect("Error loading Rust grammar");
//
//     let source_code = "fn test() {}";
//     let tree = parser.parse(source_code, None).unwrap();
//     // let mut walk = tree.walk();
//     let root_node = tree.root_node();
//     let mut i = 0;
//     while let Some(node) = root_node.named_child(i) {
//         let mut j = 0;
//         while let Some(n) = node.named_child(j) {
//             let a = n.range();
//             let s = &source_code[a.start_byte..a.end_byte];
//             dbg!(s);
//             dbg!(n.to_sexp());
//             dbg!(n.kind());
//             dbg!(n);
//             j += 1;
//         }
//         i += 1;
//     }
//
//     // dbg!(root_node.to_sexp());
//     // // let mut walk = root_node.walk();
//     // let children = root_node.children(&mut walk);
//     // for n in children {
//     //     dbg!(n.to_sexp());
//     //     dbg!(n.kind());
//     //     let mut walk = n.walk();
//     //     for c in n.named_children(&mut walk) {
//     //         // let mut walk = c.walk();
//     //         // dbg!(walk.field_name());
//     //         dbg!(c.to_sexp());
//     //     }
//     // }
//     // dbg!(&root_node);
//     // assert!(!root_node.has_error());
//     // let a = root_node.walk();
//     // dbg!(a.field_name());
//     // // root_node.language();
//     //
//     // assert_eq!(root_node.kind(), "source_file");
//     // assert_eq!(root_node.start_position().column, 0);
//     // assert_eq!(root_node.end_position().column, 12);
// }
