use alloc::{vec, vec::Vec};

use crate::{
    engine::{write_size_with_quote, BytePredicate, Error, ParserEngine, VecBuffer},
    CSV_QUOTE,
};

#[cfg(not(panic = "abort"))]
use crate::engine::quoted_bytes;

#[cfg(not(panic = "abort"))]
use std::io::{self, Write};

#[derive(Clone, Debug)]
pub struct Parser {
    engine: ParserEngine<VecBuffer>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Parser {
    pub fn new(options: u8) -> Self {
        Self {
            engine: ParserEngine::new(options),
        }
    }

    pub fn error(&self) -> Error {
        self.engine.error()
    }

    pub fn options(&self) -> u8 {
        self.engine.options()
    }

    pub fn set_options(&mut self, options: u8) {
        self.engine.set_options(options);
    }

    pub fn set_delimiter(&mut self, delimiter: u8) {
        self.engine.set_delimiter(delimiter);
    }

    pub fn set_quote(&mut self, quote: u8) {
        self.engine.set_quote(quote);
    }

    pub fn delimiter(&self) -> u8 {
        self.engine.delimiter()
    }

    pub fn quote(&self) -> u8 {
        self.engine.quote()
    }

    pub fn set_space_predicate(&mut self, predicate: Option<BytePredicate>) {
        self.engine.set_space_predicate(predicate);
    }

    pub fn set_term_predicate(&mut self, predicate: Option<BytePredicate>) {
        self.engine.set_term_predicate(predicate);
    }

    pub fn set_block_size(&mut self, size: usize) {
        self.engine.set_block_size(size);
    }

    pub fn buffer_size(&self) -> usize {
        self.engine.buffer_size()
    }

    pub fn free(&mut self) {
        self.engine.free();
    }

    pub fn parse<F1, F2>(&mut self, input: &[u8], field_cb: &mut F1, row_cb: &mut F2) -> usize
    where
        F1: FnMut(Option<&[u8]>),
        F2: FnMut(i32),
    {
        self.engine.parse(input, field_cb, row_cb)
    }

    pub fn finish<F1, F2>(&mut self, field_cb: &mut F1, row_cb: &mut F2) -> Result<(), Error>
    where
        F1: FnMut(Option<&[u8]>),
        F2: FnMut(i32),
    {
        self.engine.finish(field_cb, row_cb)
    }
}

pub fn write_to_buffer(dest: &mut [u8], src: &[u8]) -> usize {
    write_to_buffer_with_quote(dest, src, CSV_QUOTE)
}

pub fn write_to_buffer_with_quote(dest: &mut [u8], src: &[u8], quote: u8) -> usize {
    crate::engine::write_to_buffer_with_quote(dest, src, quote)
}

pub fn write(src: &[u8]) -> Vec<u8> {
    write_with_quote(src, CSV_QUOTE)
}

pub fn write_with_quote(src: &[u8], quote: u8) -> Vec<u8> {
    let mut dest = vec![0; write_size_with_quote(src, quote)];
    let actual_len = write_to_buffer_with_quote(&mut dest, src, quote);
    dest.truncate(actual_len);
    dest
}

#[cfg(not(panic = "abort"))]
pub fn fwrite<W: Write>(writer: &mut W, src: &[u8]) -> io::Result<()> {
    fwrite_with_quote(writer, src, CSV_QUOTE)
}

#[cfg(not(panic = "abort"))]
pub fn fwrite_with_quote<W: Write>(writer: &mut W, src: &[u8], quote: u8) -> io::Result<()> {
    for byte in quoted_bytes(src, quote) {
        writer.write_all(&[byte])?;
    }
    Ok(())
}
