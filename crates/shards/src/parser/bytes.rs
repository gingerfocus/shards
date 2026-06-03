use crate::prelude::*;

#[derive(Debug)]
pub struct ByteCode {
    // operations: Vec<Thing>,
}

impl From<OpCode> for ByteCode {
    fn from(value: OpCode) -> Self {
        dbg!(value);
        Self {
            // operations: Vec::new(),
        }
    }
}

// use std::marker::PhantomData;
// type Thing = PhantomData<String>;
