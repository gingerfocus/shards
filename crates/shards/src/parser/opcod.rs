//! the main logic of the starship thing
//! the two main points of interest anre the from meathod that walks the ast
//! and the reduce meathod that removes unessisary memory copies

use crate::prelude::*;

#[derive(Debug)]
pub struct OpCode {
    // pub ops: Vec<OpcodeOperation>,
}

impl From<Ast> for OpCode {
    fn from(ast: Ast) -> Self {
        //     for tok in ast.tokens {
        //         match tok {
        //             Token::Identifier(i) => match i {
        //                 Identifier::Variable {
        //                     // name,
        //                     variable_type,
        //                 } => {
        //                     // println!("var_name: {}", name);
        //                 }
        //                 _ => {}
        //             },
        //             Token::Operation(o) => match o {
        //                 Operation::ScriptCall => {
        //                     log::info!("Found a script call");
        //                 }
        //                 _ => {}
        //             },
        //         }
        //     }
        dbg!(ast);

        OpCode {
            // ops: Vec::new()
        }
    }
}

impl OpCode {
    /// Finds reduntant memory copies that can be removed while guaranteeing
    /// correctness.
    pub fn reduce(&mut self) {
        log::error!("Reduction Unimplemented.");
    }
}

// pub struct OpcodeOperation {
//     pub command: Command,
//     pub arg1: Argument,
//     pub arg2: Option<Argument>,
// }

// pub enum Command {
//     Add,
//     Subtract,
// }
//
// pub enum Argument {
//     Number(f64),
//     String(String),
//     Bool(bool),
//     Table(Table),
// }
//
// pub struct Table {
//     pub members: Vec<Value>,
// }
//
// pub enum Value {
//     KeyValue(Pair),
//     Array(Vec<Argument>),
//     Literal(Argument),
// }
//
// pub struct Pair {
//     key: Argument,
//     value: Argument,
// }
