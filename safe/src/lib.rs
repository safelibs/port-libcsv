extern crate alloc;

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

pub mod engine;
pub mod rust_api;

pub use crate::engine::{Error, strerror};
pub use crate::rust_api::{
    Parser, fwrite, fwrite_with_quote, write, write_to_buffer, write_to_buffer_with_quote,
    write_with_quote,
};
