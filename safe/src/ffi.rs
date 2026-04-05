use core::{
    ffi::{c_char, c_int, c_uchar, c_void},
    ptr::{self, NonNull},
    slice,
};

use crate::{
    engine::{quoted_bytes, write_to_buffer_with_quote},
    CSV_APPEND_NULL, CSV_COMMA, CSV_CR, CSV_EMPTY_IS_NULL, CSV_ENOMEM, CSV_EPARSE, CSV_ETOOBIG,
    CSV_LF, CSV_QUOTE, CSV_REPALL_NL, CSV_SPACE, CSV_STRICT, CSV_STRICT_FINI, CSV_SUCCESS, CSV_TAB,
    END_OF_INPUT,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
core::arch::global_asm!(
    r#"
    .globl libcsv_base_csv_error
    .type libcsv_base_csv_error,@function
libcsv_base_csv_error:
    jmp {csv_error_impl}
    .size libcsv_base_csv_error, .-libcsv_base_csv_error
    .symver libcsv_base_csv_error,csv_error@@Base,remove

    .globl libcsv_base_csv_fini
    .type libcsv_base_csv_fini,@function
libcsv_base_csv_fini:
    jmp {csv_fini_impl}
    .size libcsv_base_csv_fini, .-libcsv_base_csv_fini
    .symver libcsv_base_csv_fini,csv_fini@@Base,remove

    .globl libcsv_base_csv_free
    .type libcsv_base_csv_free,@function
libcsv_base_csv_free:
    jmp {csv_free_impl}
    .size libcsv_base_csv_free, .-libcsv_base_csv_free
    .symver libcsv_base_csv_free,csv_free@@Base,remove

    .globl libcsv_base_csv_fwrite
    .type libcsv_base_csv_fwrite,@function
libcsv_base_csv_fwrite:
    jmp {csv_fwrite_impl}
    .size libcsv_base_csv_fwrite, .-libcsv_base_csv_fwrite
    .symver libcsv_base_csv_fwrite,csv_fwrite@@Base,remove

    .globl libcsv_base_csv_fwrite2
    .type libcsv_base_csv_fwrite2,@function
libcsv_base_csv_fwrite2:
    jmp {csv_fwrite2_impl}
    .size libcsv_base_csv_fwrite2, .-libcsv_base_csv_fwrite2
    .symver libcsv_base_csv_fwrite2,csv_fwrite2@@Base,remove

    .globl libcsv_base_csv_get_buffer_size
    .type libcsv_base_csv_get_buffer_size,@function
libcsv_base_csv_get_buffer_size:
    jmp {csv_get_buffer_size_impl}
    .size libcsv_base_csv_get_buffer_size, .-libcsv_base_csv_get_buffer_size
    .symver libcsv_base_csv_get_buffer_size,csv_get_buffer_size@@Base,remove

    .globl libcsv_base_csv_get_delim
    .type libcsv_base_csv_get_delim,@function
libcsv_base_csv_get_delim:
    jmp {csv_get_delim_impl}
    .size libcsv_base_csv_get_delim, .-libcsv_base_csv_get_delim
    .symver libcsv_base_csv_get_delim,csv_get_delim@@Base,remove

    .globl libcsv_base_csv_get_opts
    .type libcsv_base_csv_get_opts,@function
libcsv_base_csv_get_opts:
    jmp {csv_get_opts_impl}
    .size libcsv_base_csv_get_opts, .-libcsv_base_csv_get_opts
    .symver libcsv_base_csv_get_opts,csv_get_opts@@Base,remove

    .globl libcsv_base_csv_get_quote
    .type libcsv_base_csv_get_quote,@function
libcsv_base_csv_get_quote:
    jmp {csv_get_quote_impl}
    .size libcsv_base_csv_get_quote, .-libcsv_base_csv_get_quote
    .symver libcsv_base_csv_get_quote,csv_get_quote@@Base,remove

    .globl libcsv_base_csv_init
    .type libcsv_base_csv_init,@function
libcsv_base_csv_init:
    jmp {csv_init_impl}
    .size libcsv_base_csv_init, .-libcsv_base_csv_init
    .symver libcsv_base_csv_init,csv_init@@Base,remove

    .globl libcsv_base_csv_parse
    .type libcsv_base_csv_parse,@function
libcsv_base_csv_parse:
    jmp {csv_parse_impl}
    .size libcsv_base_csv_parse, .-libcsv_base_csv_parse
    .symver libcsv_base_csv_parse,csv_parse@@Base,remove

    .globl libcsv_base_csv_set_blk_size
    .type libcsv_base_csv_set_blk_size,@function
libcsv_base_csv_set_blk_size:
    jmp {csv_set_blk_size_impl}
    .size libcsv_base_csv_set_blk_size, .-libcsv_base_csv_set_blk_size
    .symver libcsv_base_csv_set_blk_size,csv_set_blk_size@@Base,remove

    .globl libcsv_base_csv_set_delim
    .type libcsv_base_csv_set_delim,@function
libcsv_base_csv_set_delim:
    jmp {csv_set_delim_impl}
    .size libcsv_base_csv_set_delim, .-libcsv_base_csv_set_delim
    .symver libcsv_base_csv_set_delim,csv_set_delim@@Base,remove

    .globl libcsv_base_csv_set_free_func
    .type libcsv_base_csv_set_free_func,@function
libcsv_base_csv_set_free_func:
    jmp {csv_set_free_func_impl}
    .size libcsv_base_csv_set_free_func, .-libcsv_base_csv_set_free_func
    .symver libcsv_base_csv_set_free_func,csv_set_free_func@@Base,remove

    .globl libcsv_base_csv_set_opts
    .type libcsv_base_csv_set_opts,@function
libcsv_base_csv_set_opts:
    jmp {csv_set_opts_impl}
    .size libcsv_base_csv_set_opts, .-libcsv_base_csv_set_opts
    .symver libcsv_base_csv_set_opts,csv_set_opts@@Base,remove

    .globl libcsv_base_csv_set_quote
    .type libcsv_base_csv_set_quote,@function
libcsv_base_csv_set_quote:
    jmp {csv_set_quote_impl}
    .size libcsv_base_csv_set_quote, .-libcsv_base_csv_set_quote
    .symver libcsv_base_csv_set_quote,csv_set_quote@@Base,remove

    .globl libcsv_base_csv_set_realloc_func
    .type libcsv_base_csv_set_realloc_func,@function
libcsv_base_csv_set_realloc_func:
    jmp {csv_set_realloc_func_impl}
    .size libcsv_base_csv_set_realloc_func, .-libcsv_base_csv_set_realloc_func
    .symver libcsv_base_csv_set_realloc_func,csv_set_realloc_func@@Base,remove

    .globl libcsv_base_csv_set_space_func
    .type libcsv_base_csv_set_space_func,@function
libcsv_base_csv_set_space_func:
    jmp {csv_set_space_func_impl}
    .size libcsv_base_csv_set_space_func, .-libcsv_base_csv_set_space_func
    .symver libcsv_base_csv_set_space_func,csv_set_space_func@@Base,remove

    .globl libcsv_base_csv_set_term_func
    .type libcsv_base_csv_set_term_func,@function
libcsv_base_csv_set_term_func:
    jmp {csv_set_term_func_impl}
    .size libcsv_base_csv_set_term_func, .-libcsv_base_csv_set_term_func
    .symver libcsv_base_csv_set_term_func,csv_set_term_func@@Base,remove

    .globl libcsv_base_csv_strerror
    .type libcsv_base_csv_strerror,@function
libcsv_base_csv_strerror:
    jmp {csv_strerror_impl}
    .size libcsv_base_csv_strerror, .-libcsv_base_csv_strerror
    .symver libcsv_base_csv_strerror,csv_strerror@@Base,remove

    .globl libcsv_base_csv_write
    .type libcsv_base_csv_write,@function
libcsv_base_csv_write:
    jmp {csv_write_impl}
    .size libcsv_base_csv_write, .-libcsv_base_csv_write
    .symver libcsv_base_csv_write,csv_write@@Base,remove

    .globl libcsv_base_csv_write2
    .type libcsv_base_csv_write2,@function
libcsv_base_csv_write2:
    jmp {csv_write2_impl}
    .size libcsv_base_csv_write2, .-libcsv_base_csv_write2
    .symver libcsv_base_csv_write2,csv_write2@@Base,remove
    "#,
    csv_error_impl = sym csv_error,
    csv_fini_impl = sym csv_fini,
    csv_free_impl = sym csv_free,
    csv_fwrite_impl = sym csv_fwrite,
    csv_fwrite2_impl = sym csv_fwrite2,
    csv_get_buffer_size_impl = sym csv_get_buffer_size,
    csv_get_delim_impl = sym csv_get_delim,
    csv_get_opts_impl = sym csv_get_opts,
    csv_get_quote_impl = sym csv_get_quote,
    csv_init_impl = sym csv_init,
    csv_parse_impl = sym csv_parse,
    csv_set_blk_size_impl = sym csv_set_blk_size,
    csv_set_delim_impl = sym csv_set_delim,
    csv_set_free_func_impl = sym csv_set_free_func,
    csv_set_opts_impl = sym csv_set_opts,
    csv_set_quote_impl = sym csv_set_quote,
    csv_set_realloc_func_impl = sym csv_set_realloc_func,
    csv_set_space_func_impl = sym csv_set_space_func,
    csv_set_term_func_impl = sym csv_set_term_func,
    csv_strerror_impl = sym csv_strerror,
    csv_write_impl = sym csv_write,
    csv_write2_impl = sym csv_write2,
);

#[allow(non_camel_case_types)]
pub type csv_predicate = Option<unsafe extern "C" fn(c_uchar) -> c_int>;
#[allow(non_camel_case_types)]
pub type csv_malloc_func = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
#[allow(non_camel_case_types)]
pub type csv_realloc_func = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
#[allow(non_camel_case_types)]
pub type csv_free_func = Option<unsafe extern "C" fn(*mut c_void)>;
#[allow(non_camel_case_types)]
pub type csv_cb1 = Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void)>;
#[allow(non_camel_case_types)]
pub type csv_cb2 = Option<unsafe extern "C" fn(c_int, *mut c_void)>;
#[allow(non_camel_case_types)]
pub type FILE = c_void;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct csv_parser {
    pub pstate: c_int,
    pub quoted: c_int,
    pub spaces: usize,
    pub entry_buf: *mut c_uchar,
    pub entry_pos: usize,
    pub entry_size: usize,
    pub status: c_int,
    pub options: c_uchar,
    pub quote_char: c_uchar,
    pub delim_char: c_uchar,
    pub is_space: csv_predicate,
    pub is_term: csv_predicate,
    pub blk_size: usize,
    pub malloc_func: csv_malloc_func,
    pub realloc_func: csv_realloc_func,
    pub free_func: csv_free_func,
}

const ROW_NOT_BEGUN: c_int = 0;
const FIELD_NOT_BEGUN: c_int = 1;
const FIELD_BEGUN: c_int = 2;
const FIELD_MIGHT_HAVE_ENDED: c_int = 3;
const MEM_BLK_SIZE: usize = 128;
const EOF: c_int = -1;

const STR_SUCCESS: &[u8] = b"success\0";
const STR_EPARSE: &[u8] = b"error parsing data while strict checking enabled\0";
const STR_ENOMEM: &[u8] = b"memory exhausted while increasing buffer size\0";
const STR_ETOOBIG: &[u8] = b"data size too large\0";
const STR_EINVALID: &[u8] = b"invalid status code\0";

extern "C" {
    fn abort() -> !;
    fn free(ptr: *mut c_void);
    fn fputc(ch: c_int, stream: *mut FILE) -> c_int;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

unsafe extern "C" fn default_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(ptr, size) }
}

unsafe extern "C" fn default_free(ptr: *mut c_void) {
    unsafe { free(ptr) };
}

#[inline(always)]
fn abort_process() -> ! {
    unsafe { abort() }
}

fn require_mut_parser(ptr: *mut csv_parser) -> NonNull<csv_parser> {
    match NonNull::new(ptr) {
        Some(ptr) => ptr,
        None => abort_process(),
    }
}

fn raw_bytes<'a>(ptr: *const c_void, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        let ptr = match NonNull::new(ptr.cast_mut()) {
            Some(ptr) => ptr,
            None => abort_process(),
        };
        unsafe { slice::from_raw_parts(ptr.as_ptr().cast::<u8>(), len) }
    }
}

fn raw_bytes_mut<'a>(ptr: *mut c_void, len: usize) -> &'a mut [u8] {
    if len == 0 {
        &mut []
    } else {
        let ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => abort_process(),
        };
        unsafe { slice::from_raw_parts_mut(ptr.as_ptr().cast::<u8>(), len) }
    }
}

fn bool_to_c_int(value: bool) -> c_int {
    if value {
        1
    } else {
        0
    }
}

unsafe fn is_space_byte(predicate: csv_predicate, byte: u8) -> bool {
    match predicate {
        Some(predicate) => unsafe { predicate(byte) != 0 },
        None => byte == CSV_SPACE || byte == CSV_TAB,
    }
}

unsafe fn is_term_byte(predicate: csv_predicate, byte: u8) -> bool {
    match predicate {
        Some(predicate) => unsafe { predicate(byte) != 0 },
        None => byte == CSV_CR || byte == CSV_LF,
    }
}

unsafe fn increase_buffer(parser: &mut csv_parser) -> Result<(), ()> {
    let mut to_add = parser.blk_size;

    if parser.entry_size >= usize::MAX - to_add {
        to_add = usize::MAX - parser.entry_size;
    }

    if to_add == 0 {
        parser.status = c_int::from(CSV_ETOOBIG);
        return Err(());
    }

    let realloc_func = match parser.realloc_func {
        Some(realloc_func) => realloc_func,
        None => abort_process(),
    };

    loop {
        let new_ptr = unsafe {
            realloc_func(
                parser.entry_buf.cast::<c_void>(),
                parser.entry_size.saturating_add(to_add),
            )
        };

        if !new_ptr.is_null() {
            parser.entry_buf = new_ptr.cast::<c_uchar>();
            parser.entry_size = parser.entry_size.saturating_add(to_add);
            return Ok(());
        }

        to_add /= 2;
        if to_add == 0 {
            parser.status = c_int::from(CSV_ENOMEM);
            return Err(());
        }
    }
}

unsafe fn set_entry_byte(parser: &mut csv_parser, index: usize, byte: u8) {
    unsafe { parser.entry_buf.add(index).write(byte) };
}

unsafe fn submit_char(parser: &mut csv_parser, entry_pos: &mut usize, byte: u8) {
    unsafe { set_entry_byte(parser, *entry_pos, byte) };
    *entry_pos += 1;
}

unsafe fn submit_field(
    parser: &mut csv_parser,
    quoted: &mut bool,
    pstate: &mut c_int,
    spaces: &mut usize,
    entry_pos: &mut usize,
    cb1: csv_cb1,
    data: *mut c_void,
) {
    if !*quoted {
        *entry_pos -= *spaces;
    }

    if parser.options & CSV_APPEND_NULL != 0 {
        unsafe { set_entry_byte(parser, *entry_pos, 0) };
    }

    if let Some(cb1) = cb1 {
        if parser.options & CSV_EMPTY_IS_NULL != 0 && !*quoted && *entry_pos == 0 {
            unsafe { cb1(ptr::null_mut(), *entry_pos, data) };
        } else {
            unsafe { cb1(parser.entry_buf.cast::<c_void>(), *entry_pos, data) };
        }
    }

    *pstate = FIELD_NOT_BEGUN;
    *entry_pos = 0;
    *quoted = false;
    *spaces = 0;
}

unsafe fn submit_row(
    pstate: &mut c_int,
    quoted: &mut bool,
    spaces: &mut usize,
    entry_pos: &mut usize,
    cb2: csv_cb2,
    data: *mut c_void,
    term: c_int,
) {
    if let Some(cb2) = cb2 {
        unsafe { cb2(term, data) };
    }

    *pstate = ROW_NOT_BEGUN;
    *entry_pos = 0;
    *quoted = false;
    *spaces = 0;
}

fn save_parser_state(
    parser: &mut csv_parser,
    quoted: bool,
    pstate: c_int,
    spaces: usize,
    entry_pos: usize,
) {
    parser.quoted = bool_to_c_int(quoted);
    parser.pstate = pstate;
    parser.spaces = spaces;
    parser.entry_pos = entry_pos;
}

pub unsafe extern "C" fn csv_error(parser: *mut csv_parser) -> c_int {
    unsafe { require_mut_parser(parser).as_ref().status }
}

pub extern "C" fn csv_strerror(status: c_int) -> *mut c_char {
    let message = match status {
        value if value == c_int::from(CSV_SUCCESS) => STR_SUCCESS,
        value if value == c_int::from(CSV_EPARSE) => STR_EPARSE,
        value if value == c_int::from(CSV_ENOMEM) => STR_ENOMEM,
        value if value == c_int::from(CSV_ETOOBIG) => STR_ETOOBIG,
        _ => STR_EINVALID,
    };

    message.as_ptr().cast_mut().cast::<c_char>()
}

pub unsafe extern "C" fn csv_get_opts(parser: *mut csv_parser) -> c_int {
    if parser.is_null() {
        -1
    } else {
        unsafe { require_mut_parser(parser).as_ref().options.into() }
    }
}

pub unsafe extern "C" fn csv_set_opts(parser: *mut csv_parser, options: c_uchar) -> c_int {
    if parser.is_null() {
        -1
    } else {
        unsafe { require_mut_parser(parser).as_mut().options = options };
        0
    }
}

pub unsafe extern "C" fn csv_init(parser: *mut csv_parser, options: c_uchar) -> c_int {
    let mut parser = match NonNull::new(parser) {
        Some(parser) => parser,
        None => return -1,
    };

    let parser = unsafe { parser.as_mut() };
    parser.entry_buf = ptr::null_mut();
    parser.pstate = ROW_NOT_BEGUN;
    parser.quoted = 0;
    parser.spaces = 0;
    parser.entry_pos = 0;
    parser.entry_size = 0;
    parser.status = c_int::from(CSV_SUCCESS);
    parser.options = options;
    parser.quote_char = CSV_QUOTE;
    parser.delim_char = CSV_COMMA;
    parser.is_space = None;
    parser.is_term = None;
    parser.blk_size = MEM_BLK_SIZE;
    parser.malloc_func = None;
    parser.realloc_func = Some(default_realloc);
    parser.free_func = Some(default_free);

    0
}

pub unsafe extern "C" fn csv_free(parser: *mut csv_parser) {
    let mut parser = match NonNull::new(parser) {
        Some(parser) => parser,
        None => return,
    };

    let parser = unsafe { parser.as_mut() };
    if !parser.entry_buf.is_null() {
        let free_func = match parser.free_func {
            Some(free_func) => free_func,
            None => abort_process(),
        };
        unsafe { free_func(parser.entry_buf.cast::<c_void>()) };
    }

    parser.entry_buf = ptr::null_mut();
    parser.entry_size = 0;
}

pub unsafe extern "C" fn csv_fini(
    parser: *mut csv_parser,
    cb1: csv_cb1,
    cb2: csv_cb2,
    data: *mut c_void,
) -> c_int {
    let parser = unsafe { require_mut_parser(parser).as_mut() };

    let mut quoted = parser.quoted != 0;
    let mut pstate = parser.pstate;
    let mut spaces = parser.spaces;
    let mut entry_pos = parser.entry_pos;

    if parser.pstate == FIELD_BEGUN
        && parser.quoted != 0
        && parser.options & CSV_STRICT != 0
        && parser.options & CSV_STRICT_FINI != 0
    {
        parser.status = c_int::from(CSV_EPARSE);
        return -1;
    }

    match parser.pstate {
        FIELD_MIGHT_HAVE_ENDED => {
            entry_pos -= spaces + 1;
            unsafe {
                submit_field(
                    parser,
                    &mut quoted,
                    &mut pstate,
                    &mut spaces,
                    &mut entry_pos,
                    cb1,
                    data,
                );
                submit_row(
                    &mut pstate,
                    &mut quoted,
                    &mut spaces,
                    &mut entry_pos,
                    cb2,
                    data,
                    END_OF_INPUT,
                );
            }
        }
        FIELD_NOT_BEGUN | FIELD_BEGUN => unsafe {
            submit_field(
                parser,
                &mut quoted,
                &mut pstate,
                &mut spaces,
                &mut entry_pos,
                cb1,
                data,
            );
            submit_row(
                &mut pstate,
                &mut quoted,
                &mut spaces,
                &mut entry_pos,
                cb2,
                data,
                END_OF_INPUT,
            );
        },
        ROW_NOT_BEGUN => {}
        _ => {}
    }

    parser.spaces = 0;
    parser.quoted = 0;
    parser.entry_pos = 0;
    parser.status = c_int::from(CSV_SUCCESS);
    parser.pstate = ROW_NOT_BEGUN;
    0
}

pub unsafe extern "C" fn csv_set_delim(parser: *mut csv_parser, value: c_uchar) {
    if let Some(mut parser) = NonNull::new(parser) {
        unsafe { parser.as_mut().delim_char = value };
    }
}

pub unsafe extern "C" fn csv_set_quote(parser: *mut csv_parser, value: c_uchar) {
    if let Some(mut parser) = NonNull::new(parser) {
        unsafe { parser.as_mut().quote_char = value };
    }
}

pub unsafe extern "C" fn csv_get_delim(parser: *mut csv_parser) -> c_uchar {
    unsafe { require_mut_parser(parser).as_ref().delim_char }
}

pub unsafe extern "C" fn csv_get_quote(parser: *mut csv_parser) -> c_uchar {
    unsafe { require_mut_parser(parser).as_ref().quote_char }
}

pub unsafe extern "C" fn csv_set_space_func(parser: *mut csv_parser, predicate: csv_predicate) {
    if let Some(mut parser) = NonNull::new(parser) {
        unsafe { parser.as_mut().is_space = predicate };
    }
}

pub unsafe extern "C" fn csv_set_term_func(parser: *mut csv_parser, predicate: csv_predicate) {
    if let Some(mut parser) = NonNull::new(parser) {
        unsafe { parser.as_mut().is_term = predicate };
    }
}

pub unsafe extern "C" fn csv_set_realloc_func(
    parser: *mut csv_parser,
    realloc_func: csv_realloc_func,
) {
    if let (Some(mut parser), Some(realloc_func)) = (NonNull::new(parser), realloc_func) {
        unsafe { parser.as_mut().realloc_func = Some(realloc_func) };
    }
}

pub unsafe extern "C" fn csv_set_free_func(parser: *mut csv_parser, free_func: csv_free_func) {
    if let (Some(mut parser), Some(free_func)) = (NonNull::new(parser), free_func) {
        unsafe { parser.as_mut().free_func = Some(free_func) };
    }
}

pub unsafe extern "C" fn csv_set_blk_size(parser: *mut csv_parser, size: usize) {
    if let Some(mut parser) = NonNull::new(parser) {
        unsafe { parser.as_mut().blk_size = size };
    }
}

pub unsafe extern "C" fn csv_get_buffer_size(parser: *mut csv_parser) -> usize {
    match NonNull::new(parser) {
        Some(parser) => unsafe { parser.as_ref().entry_size },
        None => 0,
    }
}

pub unsafe extern "C" fn csv_parse(
    parser: *mut csv_parser,
    input: *const c_void,
    len: usize,
    cb1: csv_cb1,
    cb2: csv_cb2,
    data: *mut c_void,
) -> usize {
    let parser = unsafe { require_mut_parser(parser).as_mut() };
    let input = raw_bytes(input, len);
    let mut pos = 0usize;
    let delim = parser.delim_char;
    let quote = parser.quote_char;
    let is_space = parser.is_space;
    let is_term = parser.is_term;
    let mut quoted = parser.quoted != 0;
    let mut pstate = parser.pstate;
    let mut spaces = parser.spaces;
    let mut entry_pos = parser.entry_pos;

    if parser.entry_buf.is_null()
        && pos < input.len()
        && unsafe { increase_buffer(parser) }.is_err()
    {
        save_parser_state(parser, quoted, pstate, spaces, entry_pos);
        return pos;
    }

    while pos < input.len() {
        let limit = if parser.options & CSV_APPEND_NULL != 0 {
            parser.entry_size - 1
        } else {
            parser.entry_size
        };

        if entry_pos == limit && unsafe { increase_buffer(parser) }.is_err() {
            save_parser_state(parser, quoted, pstate, spaces, entry_pos);
            return pos;
        }

        let byte = input[pos];
        pos += 1;

        match pstate {
            ROW_NOT_BEGUN | FIELD_NOT_BEGUN => {
                if unsafe { is_space_byte(is_space, byte) } && byte != delim {
                    continue;
                }

                if unsafe { is_term_byte(is_term, byte) } {
                    if pstate == FIELD_NOT_BEGUN {
                        unsafe {
                            submit_field(
                                parser,
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                cb1,
                                data,
                            );
                            submit_row(
                                &mut pstate,
                                &mut quoted,
                                &mut spaces,
                                &mut entry_pos,
                                cb2,
                                data,
                                c_int::from(byte),
                            );
                        }
                    } else if parser.options & CSV_REPALL_NL != 0 {
                        unsafe {
                            submit_row(
                                &mut pstate,
                                &mut quoted,
                                &mut spaces,
                                &mut entry_pos,
                                cb2,
                                data,
                                c_int::from(byte),
                            );
                        }
                    }
                    continue;
                }

                if byte == delim {
                    unsafe {
                        submit_field(
                            parser,
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            cb1,
                            data,
                        );
                    }
                } else if byte == quote {
                    pstate = FIELD_BEGUN;
                    quoted = true;
                } else {
                    pstate = FIELD_BEGUN;
                    quoted = false;
                    unsafe { submit_char(parser, &mut entry_pos, byte) };
                }
            }
            FIELD_BEGUN => {
                if byte == quote {
                    if quoted {
                        unsafe { submit_char(parser, &mut entry_pos, byte) };
                        pstate = FIELD_MIGHT_HAVE_ENDED;
                    } else {
                        if parser.options & CSV_STRICT != 0 {
                            parser.status = c_int::from(CSV_EPARSE);
                            save_parser_state(parser, quoted, pstate, spaces, entry_pos);
                            return pos - 1;
                        }
                        unsafe { submit_char(parser, &mut entry_pos, byte) };
                        spaces = 0;
                    }
                } else if byte == delim {
                    if quoted {
                        unsafe { submit_char(parser, &mut entry_pos, byte) };
                    } else {
                        unsafe {
                            submit_field(
                                parser,
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                cb1,
                                data,
                            );
                        }
                    }
                } else if unsafe { is_term_byte(is_term, byte) } {
                    if quoted {
                        unsafe { submit_char(parser, &mut entry_pos, byte) };
                    } else {
                        unsafe {
                            submit_field(
                                parser,
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                cb1,
                                data,
                            );
                            submit_row(
                                &mut pstate,
                                &mut quoted,
                                &mut spaces,
                                &mut entry_pos,
                                cb2,
                                data,
                                c_int::from(byte),
                            );
                        }
                    }
                } else if !quoted && unsafe { is_space_byte(is_space, byte) } {
                    unsafe { submit_char(parser, &mut entry_pos, byte) };
                    spaces += 1;
                } else {
                    unsafe { submit_char(parser, &mut entry_pos, byte) };
                    spaces = 0;
                }
            }
            FIELD_MIGHT_HAVE_ENDED => {
                if byte == delim {
                    entry_pos -= spaces + 1;
                    unsafe {
                        submit_field(
                            parser,
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            cb1,
                            data,
                        );
                    }
                } else if unsafe { is_term_byte(is_term, byte) } {
                    entry_pos -= spaces + 1;
                    unsafe {
                        submit_field(
                            parser,
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            cb1,
                            data,
                        );
                        submit_row(
                            &mut pstate,
                            &mut quoted,
                            &mut spaces,
                            &mut entry_pos,
                            cb2,
                            data,
                            c_int::from(byte),
                        );
                    }
                } else if unsafe { is_space_byte(is_space, byte) } {
                    unsafe { submit_char(parser, &mut entry_pos, byte) };
                    spaces += 1;
                } else if byte == quote {
                    if spaces != 0 {
                        if parser.options & CSV_STRICT != 0 {
                            parser.status = c_int::from(CSV_EPARSE);
                            save_parser_state(parser, quoted, pstate, spaces, entry_pos);
                            return pos - 1;
                        }
                        spaces = 0;
                        unsafe { submit_char(parser, &mut entry_pos, byte) };
                    } else {
                        pstate = FIELD_BEGUN;
                    }
                } else {
                    if parser.options & CSV_STRICT != 0 {
                        parser.status = c_int::from(CSV_EPARSE);
                        save_parser_state(parser, quoted, pstate, spaces, entry_pos);
                        return pos - 1;
                    }
                    pstate = FIELD_BEGUN;
                    spaces = 0;
                    unsafe { submit_char(parser, &mut entry_pos, byte) };
                }
            }
            _ => {}
        }
    }

    save_parser_state(parser, quoted, pstate, spaces, entry_pos);
    pos
}

pub unsafe extern "C" fn csv_write(
    dest: *mut c_void,
    dest_size: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    unsafe { csv_write2(dest, dest_size, src, src_size, CSV_QUOTE) }
}

pub unsafe extern "C" fn csv_fwrite(fp: *mut FILE, src: *const c_void, src_size: usize) -> c_int {
    unsafe { csv_fwrite2(fp, src, src_size, CSV_QUOTE) }
}

pub unsafe extern "C" fn csv_write2(
    dest: *mut c_void,
    dest_size: usize,
    src: *const c_void,
    src_size: usize,
    quote: c_uchar,
) -> usize {
    if src.is_null() {
        return 0;
    }

    let src = raw_bytes(src, src_size);
    let dest = if dest.is_null() {
        &mut []
    } else {
        raw_bytes_mut(dest, dest_size)
    };

    write_to_buffer_with_quote(dest, src, quote)
}

pub unsafe extern "C" fn csv_fwrite2(
    fp: *mut FILE,
    src: *const c_void,
    src_size: usize,
    quote: c_uchar,
) -> c_int {
    if fp.is_null() || src.is_null() {
        return 0;
    }

    let src = raw_bytes(src, src_size);
    for byte in quoted_bytes(src, quote) {
        if unsafe { fputc(c_int::from(byte), fp) } == EOF {
            return EOF;
        }
    }

    0
}
