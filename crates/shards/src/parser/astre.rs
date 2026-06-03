use core::fmt;
use std::ffi::OsStr;

use crate::prelude::*;
use libloading::{Library, Symbol};

// impl Iterator for Ast {
//     type Item = syn::File;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         Some(self.args.clone())
//     }
// }

// pub struct Ast<'a> {
//     args: VecDeque<&'a str>,
// }
//
// impl<'a> Ast<'a> {
//     pub fn parse(s: &'a str, _sys: &SystemState) -> anyhow::Result<Ast<'a>> {
//         Ok(Ast {
//             args: s.split_whitespace().collect::<VecDeque<&str>>(),
//         })
//     }
//
//     pub fn next(&mut self) -> Option<&'a str> {
//         self.args.pop_front()
//     }
//
//     pub fn get_args(&mut self) -> Vec<&'a str> {
//         let ret = self.args.iter().map(|s| *s).collect();
//         self.args.clear();
//         ret
//     }
// }

#[derive(Debug)]
pub enum LoaderError {
    NoCdylib,
    NoSymbol,
}
impl fmt::Display for LoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoaderError::NoCdylib => f.write_str("library could not be opened"),
            LoaderError::NoSymbol => f.write_str("could not get the `parse` fn"),
        }
    }
}
impl Context for LoaderError {}

#[derive(Debug)]
pub struct Loader {
    _lib: Library,
    // func is borrowed from the above. It will be loaded as long as the above
    // is loaded. This means there can be a very brief time when droping the
    // object which this may point to an unloaded library, however as long as
    // drop is not implemented for this it should be fine.
    func: Symbol<'static, libshards::ParseFuncSig>,
}

static PARSE: [u8; 6] = *b"parse\0";

impl Loader {
    pub fn new(path: impl AsRef<OsStr>) -> Result<Self, LoaderError> {
        // This function should be limited in what it accepts
        let (_lib, func) = unsafe {
            let lib = Library::new(path).change_context(LoaderError::NoCdylib)?;
            use libshards::ParseFuncSig;

            let func = lib
                .get::<ParseFuncSig>(&PARSE)
                .change_context(LoaderError::NoSymbol)?;

            let func = std::mem::transmute(func);
            (lib, func)
        };

        Ok(Self { _lib, func })
    }

    pub fn parse(&self, data: &str) -> Option<Ast> {
        libshards::ParseFunction::parse(&*self.func, data)
    }
}
