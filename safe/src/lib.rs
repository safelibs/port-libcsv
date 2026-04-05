use std::io::{self, Write};

pub const CSV_MAJOR: u8 = 3;
pub const CSV_MINOR: u8 = 0;
pub const CSV_RELEASE: u8 = 3;

pub const CSV_STRICT: u8 = 1;
pub const CSV_REPALL_NL: u8 = 2;
pub const CSV_STRICT_FINI: u8 = 4;
pub const CSV_APPEND_NULL: u8 = 8;
pub const CSV_EMPTY_IS_NULL: u8 = 16;

pub const CSV_TAB: u8 = 0x09;
pub const CSV_SPACE: u8 = 0x20;
pub const CSV_CR: u8 = 0x0d;
pub const CSV_LF: u8 = 0x0a;
pub const CSV_COMMA: u8 = 0x2c;
pub const CSV_QUOTE: u8 = 0x22;

pub const CSV_SUCCESS: u8 = 0;
pub const CSV_EPARSE: u8 = 1;
pub const CSV_ENOMEM: u8 = 2;
pub const CSV_ETOOBIG: u8 = 3;
pub const CSV_EINVALID: u8 = 4;
pub const END_OF_INPUT: i32 = -1;

const DEFAULT_BLOCK_SIZE: usize = 128;

type BytePredicate = fn(u8) -> bool;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Success,
    Parse,
    TooBig,
}

impl Error {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => CSV_SUCCESS,
            Self::Parse => CSV_EPARSE,
            Self::TooBig => CSV_ETOOBIG,
        }
    }
}

pub fn strerror(code: u8) -> &'static str {
    match code {
        CSV_SUCCESS => "success",
        CSV_EPARSE => "error parsing data while strict checking enabled",
        CSV_ENOMEM => "memory exhausted while increasing buffer size",
        CSV_ETOOBIG => "data size too large",
        _ => "invalid status code",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParserState {
    RowNotBegun,
    FieldNotBegun,
    FieldBegun,
    FieldMightHaveEnded,
}

#[derive(Debug, Clone)]
pub struct Parser {
    pstate: ParserState,
    quoted: bool,
    spaces: usize,
    entry_buf: Vec<u8>,
    entry_pos: usize,
    status: Error,
    options: u8,
    quote_char: u8,
    delim_char: u8,
    is_space: Option<BytePredicate>,
    is_term: Option<BytePredicate>,
    blk_size: usize,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Parser {
    pub fn new(options: u8) -> Self {
        Self {
            pstate: ParserState::RowNotBegun,
            quoted: false,
            spaces: 0,
            entry_buf: Vec::new(),
            entry_pos: 0,
            status: Error::Success,
            options,
            quote_char: CSV_QUOTE,
            delim_char: CSV_COMMA,
            is_space: None,
            is_term: None,
            blk_size: DEFAULT_BLOCK_SIZE,
        }
    }

    pub fn error(&self) -> Error {
        self.status
    }

    pub fn options(&self) -> u8 {
        self.options
    }

    pub fn set_options(&mut self, options: u8) {
        self.options = options;
    }

    pub fn set_delimiter(&mut self, delimiter: u8) {
        self.delim_char = delimiter;
    }

    pub fn set_quote(&mut self, quote: u8) {
        self.quote_char = quote;
    }

    pub fn delimiter(&self) -> u8 {
        self.delim_char
    }

    pub fn quote(&self) -> u8 {
        self.quote_char
    }

    pub fn set_space_predicate(&mut self, predicate: Option<BytePredicate>) {
        self.is_space = predicate;
    }

    pub fn set_term_predicate(&mut self, predicate: Option<BytePredicate>) {
        self.is_term = predicate;
    }

    pub fn set_block_size(&mut self, size: usize) {
        self.blk_size = size;
    }

    pub fn buffer_size(&self) -> usize {
        self.entry_buf.len()
    }

    pub fn free(&mut self) {
        self.entry_buf = Vec::new();
    }

    fn increase_buffer(&mut self) -> Result<(), Error> {
        if self.blk_size == 0 {
            self.status = Error::TooBig;
            return Err(Error::TooBig);
        }

        let new_len = self
            .entry_buf
            .len()
            .checked_add(self.blk_size)
            .ok_or(Error::TooBig)?;
        self.entry_buf.resize(new_len, 0);
        Ok(())
    }

    fn is_space(&self, byte: u8) -> bool {
        self.is_space
            .map_or(byte == CSV_SPACE || byte == CSV_TAB, |predicate| {
                predicate(byte)
            })
    }

    fn is_term(&self, byte: u8) -> bool {
        self.is_term
            .map_or(byte == CSV_CR || byte == CSV_LF, |predicate| {
                predicate(byte)
            })
    }

    fn ensure_capacity(&mut self) -> Result<(), Error> {
        let limit = if self.options & CSV_APPEND_NULL != 0 {
            self.entry_buf.len().saturating_sub(1)
        } else {
            self.entry_buf.len()
        };

        if self.entry_pos == limit {
            self.increase_buffer()?;
        }

        Ok(())
    }

    fn submit_char(&mut self, byte: u8) {
        self.entry_buf[self.entry_pos] = byte;
        self.entry_pos += 1;
    }

    fn submit_field<F>(&mut self, field_cb: &mut F)
    where
        F: FnMut(Option<&[u8]>),
    {
        if !self.quoted {
            self.entry_pos = self.entry_pos.saturating_sub(self.spaces);
        }

        if self.options & CSV_APPEND_NULL != 0 && self.entry_pos < self.entry_buf.len() {
            self.entry_buf[self.entry_pos] = 0;
        }

        let should_emit_null =
            self.options & CSV_EMPTY_IS_NULL != 0 && !self.quoted && self.entry_pos == 0;

        if should_emit_null {
            field_cb(None);
        } else {
            field_cb(Some(&self.entry_buf[..self.entry_pos]));
        }

        self.pstate = ParserState::FieldNotBegun;
        self.entry_pos = 0;
        self.quoted = false;
        self.spaces = 0;
    }

    fn submit_row<F>(&mut self, row_cb: &mut F, term: i32)
    where
        F: FnMut(i32),
    {
        row_cb(term);
        self.pstate = ParserState::RowNotBegun;
        self.entry_pos = 0;
        self.quoted = false;
        self.spaces = 0;
    }

    pub fn parse<F1, F2>(&mut self, input: &[u8], field_cb: &mut F1, row_cb: &mut F2) -> usize
    where
        F1: FnMut(Option<&[u8]>),
        F2: FnMut(i32),
    {
        let mut pos = 0;

        if self.entry_buf.is_empty() && !input.is_empty() && self.increase_buffer().is_err() {
            return 0;
        }

        while pos < input.len() {
            if self.ensure_capacity().is_err() {
                return pos;
            }

            let byte = input[pos];
            pos += 1;

            match self.pstate {
                ParserState::RowNotBegun | ParserState::FieldNotBegun => {
                    if self.is_space(byte) && byte != self.delim_char {
                        continue;
                    }

                    if self.is_term(byte) {
                        if self.pstate == ParserState::FieldNotBegun {
                            self.submit_field(field_cb);
                            self.submit_row(row_cb, i32::from(byte));
                        } else if self.options & CSV_REPALL_NL != 0 {
                            self.submit_row(row_cb, i32::from(byte));
                        }
                        continue;
                    }

                    if byte == self.delim_char {
                        self.submit_field(field_cb);
                    } else if byte == self.quote_char {
                        self.pstate = ParserState::FieldBegun;
                        self.quoted = true;
                    } else {
                        self.pstate = ParserState::FieldBegun;
                        self.quoted = false;
                        self.submit_char(byte);
                    }
                }
                ParserState::FieldBegun => {
                    if byte == self.quote_char {
                        if self.quoted {
                            self.submit_char(byte);
                            self.pstate = ParserState::FieldMightHaveEnded;
                        } else if self.options & CSV_STRICT != 0 {
                            self.status = Error::Parse;
                            return pos - 1;
                        } else {
                            self.submit_char(byte);
                            self.spaces = 0;
                        }
                    } else if byte == self.delim_char {
                        if self.quoted {
                            self.submit_char(byte);
                        } else {
                            self.submit_field(field_cb);
                        }
                    } else if self.is_term(byte) {
                        if self.quoted {
                            self.submit_char(byte);
                        } else {
                            self.submit_field(field_cb);
                            self.submit_row(row_cb, i32::from(byte));
                        }
                    } else if !self.quoted && self.is_space(byte) {
                        self.submit_char(byte);
                        self.spaces += 1;
                    } else {
                        self.submit_char(byte);
                        self.spaces = 0;
                    }
                }
                ParserState::FieldMightHaveEnded => {
                    if byte == self.delim_char {
                        self.entry_pos -= self.spaces + 1;
                        self.submit_field(field_cb);
                    } else if self.is_term(byte) {
                        self.entry_pos -= self.spaces + 1;
                        self.submit_field(field_cb);
                        self.submit_row(row_cb, i32::from(byte));
                    } else if self.is_space(byte) {
                        self.submit_char(byte);
                        self.spaces += 1;
                    } else if byte == self.quote_char {
                        if self.spaces != 0 {
                            if self.options & CSV_STRICT != 0 {
                                self.status = Error::Parse;
                                return pos - 1;
                            }
                            self.spaces = 0;
                            self.submit_char(byte);
                        } else {
                            self.pstate = ParserState::FieldBegun;
                        }
                    } else if self.options & CSV_STRICT != 0 {
                        self.status = Error::Parse;
                        return pos - 1;
                    } else {
                        self.pstate = ParserState::FieldBegun;
                        self.spaces = 0;
                        self.submit_char(byte);
                    }
                }
            }
        }

        pos
    }

    pub fn finish<F1, F2>(&mut self, field_cb: &mut F1, row_cb: &mut F2) -> Result<(), Error>
    where
        F1: FnMut(Option<&[u8]>),
        F2: FnMut(i32),
    {
        if self.pstate == ParserState::FieldBegun
            && self.quoted
            && self.options & CSV_STRICT != 0
            && self.options & CSV_STRICT_FINI != 0
        {
            self.status = Error::Parse;
            return Err(Error::Parse);
        }

        match self.pstate {
            ParserState::FieldMightHaveEnded => {
                self.entry_pos -= self.spaces + 1;
                self.submit_field(field_cb);
                self.submit_row(row_cb, END_OF_INPUT);
            }
            ParserState::FieldNotBegun | ParserState::FieldBegun => {
                self.submit_field(field_cb);
                self.submit_row(row_cb, END_OF_INPUT);
            }
            ParserState::RowNotBegun => {}
        }

        self.spaces = 0;
        self.quoted = false;
        self.entry_pos = 0;
        self.status = Error::Success;
        self.pstate = ParserState::RowNotBegun;
        Ok(())
    }
}

pub fn write_to_buffer(dest: &mut [u8], src: &[u8]) -> usize {
    write_to_buffer_with_quote(dest, src, CSV_QUOTE)
}

pub fn write_to_buffer_with_quote(dest: &mut [u8], src: &[u8], quote: u8) -> usize {
    let mut chars = 0usize;

    if !dest.is_empty() {
        dest[0] = quote;
    }
    chars += 1;

    for &byte in src {
        if byte == quote {
            if dest.len() > chars {
                dest[chars] = quote;
            }
            chars += 1;
        }

        if dest.len() > chars {
            dest[chars] = byte;
        }
        chars += 1;
    }

    if dest.len() > chars {
        dest[chars] = quote;
    }
    chars += 1;

    chars
}

pub fn write(src: &[u8]) -> Vec<u8> {
    write_with_quote(src, CSV_QUOTE)
}

pub fn write_with_quote(src: &[u8], quote: u8) -> Vec<u8> {
    let mut dest = vec![0; src.len() * 2 + 2];
    let actual_len = write_to_buffer_with_quote(&mut dest, src, quote);
    dest.truncate(actual_len);
    dest
}

pub fn fwrite<W: Write>(writer: &mut W, src: &[u8]) -> io::Result<()> {
    fwrite_with_quote(writer, src, CSV_QUOTE)
}

pub fn fwrite_with_quote<W: Write>(writer: &mut W, src: &[u8], quote: u8) -> io::Result<()> {
    writer.write_all(&[quote])?;
    for &byte in src {
        if byte == quote {
            writer.write_all(&[quote])?;
        }
        writer.write_all(&[byte])?;
    }
    writer.write_all(&[quote])?;
    Ok(())
}
