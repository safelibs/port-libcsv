use std::{ffi::{c_int, c_uchar, c_void}, ptr};

#[repr(C)]
struct csv_parser {
    pstate: c_int,
    quoted: c_int,
    spaces: usize,
    entry_buf: *mut c_uchar,
    entry_pos: usize,
    entry_size: usize,
    status: c_int,
    options: c_uchar,
    quote_char: c_uchar,
    delim_char: c_uchar,
    is_space: Option<unsafe extern "C" fn(c_uchar) -> c_int>,
    is_term: Option<unsafe extern "C" fn(c_uchar) -> c_int>,
    blk_size: usize,
    malloc_func: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    realloc_func: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    free_func: Option<unsafe extern "C" fn(*mut c_void)>,
}

#[link(name = "csv")]
unsafe extern "C" {
    fn csv_free(parser: *mut csv_parser);
    fn csv_fini(
        parser: *mut csv_parser,
        cb1: Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void)>,
        cb2: Option<unsafe extern "C" fn(c_int, *mut c_void)>,
        data: *mut c_void,
    ) -> c_int;
    fn csv_init(parser: *mut csv_parser, options: c_uchar) -> c_int;
    fn csv_parse(
        parser: *mut csv_parser,
        input: *const c_void,
        len: usize,
        cb1: Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void)>,
        cb2: Option<unsafe extern "C" fn(c_int, *mut c_void)>,
        data: *mut c_void,
    ) -> usize;
}

unsafe extern "C" fn panic_tripwire(_: *mut c_void, _: usize, _: *mut c_void) {
    panic!("panic tripwire reached the callback");
}

fn main() {
    let mut parser = unsafe { std::mem::zeroed::<csv_parser>() };
    let input = [b'x'];

    unsafe {
        assert_eq!(csv_init(&mut parser, 0), 0);
        assert_eq!(
            csv_parse(
                &mut parser,
                input.as_ptr().cast::<c_void>(),
                input.len(),
                None,
                None,
                ptr::null_mut(),
            ),
            input.len(),
        );

        let rc = csv_fini(&mut parser, Some(panic_tripwire), None, ptr::null_mut());
        csv_free(&mut parser);
        eprintln!("unexpected csv_fini return value: {rc}");
        std::process::exit(1);
    }
}
