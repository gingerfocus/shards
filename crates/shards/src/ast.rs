// use crate::jit;
// use std::mem;

pub enum Expr {
    Literal(String),
    Identifier(String),
    Assign(String, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    IfElse(Box<Expr>, Vec<Expr>, Vec<Expr>),
    WhileLoop(Box<Expr>, Vec<Expr>),
    Call(String, Vec<Expr>),
    GlobalDataAddr(String),
}

// Executes the given code using the cranelift JIT compiler.
//
// Feeds the given input into the JIT compiled function and returns the resulting output.
//
// # Safety
//
// This function is unsafe since it relies on the caller to provide it with the correct
// input and output types. Using incorrect types at this point may corrupt the program's state.
// unsafe fn run_code<I, O>(jit: &mut jit::JIT, code: &str, input: I) -> Result<O, String> {
//     // Pass the string to the JIT, and it returns a raw pointer to machine code.
//     let code_ptr = jit.compile(code)?;
//     // Cast the raw pointer to a typed function pointer. This is unsafe, because
//     // this is the critical point where you have to trust that the generated code
//     // is safe to be called.
//     let code_fn = mem::transmute::<_, fn(I) -> O>(code_ptr);
//     // And now we can call it!
//     Ok(code_fn(input))
// }

// fn run_hello(jit: &mut jit::JIT) -> Result<isize, String> {
//     jit.create_data("hello_string", "hello world!\0".as_bytes().to_vec())?;
//     unsafe { run_code(jit, HELLO_CODE, ()) }
// }
//
// const HELLO_CODE: &str = "";
