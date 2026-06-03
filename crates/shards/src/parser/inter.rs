use std::marker::PhantomData;

use crate::prelude::*;

#[derive(Debug)]
pub enum InterpreterError {
    SwitchLang(Lang),
}
impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterpreterError::SwitchLang(l) => write!(f, "could not switch to lang: {:?}", l),
        }
    }
}
impl Context for InterpreterError {}

pub struct Interpreter {
    pub loader: Loader,
    // so that people can't construct this manyally
    _pd: PhantomData<()>,
}

#[derive(Debug, Clone, Copy)]
pub enum Lang {
    Rust,
    Rush,
    Zigi,
}

impl Lang {
    fn path(&self) -> &str {
        match self {
            Lang::Rust => "librushi.so",
            Lang::Rush => "librush.so",
            Lang::Zigi => "libstardust.so",
        }
    }
}

impl Interpreter {
    pub fn new(lang: Lang) -> Result<Self, InterpreterError> {
        let crate_path = env!("PWD");
        let path = format!("{}/lib/{}", crate_path, lang.path());
        let loader = Loader::new(path).change_context(InterpreterError::SwitchLang(lang))?;

        Ok(Self {
            loader,
            _pd: PhantomData,
        })
    }

    pub fn switch_lang(&mut self, lang: Lang) -> Result<(), InterpreterError> {
        let crate_path = env!("PWD");
        let path = format!("{}/lib/{}", crate_path, lang.path());
        let loader = Loader::new(path).change_context(InterpreterError::SwitchLang(lang))?;
        self.loader = loader;
        Ok(())
    }

    // TODO: make it take some sort of enviorment state so that it can resolve
    // aliases
    pub fn eval(&self, bytes: ByteCode) -> Result<(), InterpreterError> {
        eprintln!("running code.");

        dbg!(bytes);

        // match ast.next().unwrap() {
        //     "exit" => return Err(anyhow::anyhow!("exit")),
        //     "echo" => {
        //         println!("{}", ast.get_args().join(" "));
        //         return Ok(());
        //     },
        //     "set" => {
        //         // let args = ast.get_args();
        //         // builtins::set(key, args)
        //     },
        //     // p if p.is_path() => {
        //     //     // try to run the program
        //     //
        //     // }
        //     l if l.contains('=') => {
        //         // let (key, val) = l.split_once('=').unwrap();
        //         // builtins::set(key, val, env)
        //     }
        //     _ => {}
        // };

        // Err(anyhow::anyhow!("Function should have returned by now").into())
        Ok(())
    }
}
