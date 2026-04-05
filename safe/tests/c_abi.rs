use std::{
    collections::BTreeMap,
    ffi::{c_char, c_int, c_uchar, c_void, CStr},
    fs,
    mem::{align_of, size_of},
    path::{Path, PathBuf},
    process::{self, Command},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use csv::{
    ffi::{
        csv_fini, csv_free, csv_fwrite2, csv_get_buffer_size, csv_get_delim, csv_get_opts,
        csv_get_quote, csv_init, csv_parse, csv_parser, csv_set_blk_size, csv_set_delim,
        csv_set_free_func, csv_set_opts, csv_set_quote, csv_set_realloc_func, csv_set_space_func,
        csv_set_term_func, csv_strerror, csv_write, csv_write2, FILE,
    },
    CSV_APPEND_NULL, CSV_EMPTY_IS_NULL, CSV_EPARSE, CSV_QUOTE, CSV_SUCCESS, END_OF_INPUT,
};

static REALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn fclose(stream: *mut FILE) -> c_int;
    fn free(ptr: *mut c_void);
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut FILE) -> usize;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn rewind(stream: *mut FILE);
    fn tmpfile() -> *mut FILE;
}

#[derive(Debug, Default)]
struct Recorder {
    parser: *mut csv_parser,
    fields: Vec<Option<Vec<u8>>>,
    rows: Vec<c_int>,
    saw_live_buffer: bool,
    first_field_mutated: bool,
}

unsafe extern "C" fn realloc_hook(ptr: *mut c_void, size: usize) -> *mut c_void {
    REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    unsafe { realloc(ptr, size) }
}

unsafe extern "C" fn free_hook(ptr: *mut c_void) {
    FREE_CALLS.fetch_add(1, Ordering::Relaxed);
    unsafe { free(ptr) };
}

unsafe extern "C" fn underscore(byte: c_uchar) -> c_int {
    c_int::from(byte == b'_')
}

unsafe extern "C" fn semicolon(byte: c_uchar) -> c_int {
    c_int::from(byte == b';')
}

unsafe extern "C" fn field_callback(field: *mut c_void, len: usize, data: *mut c_void) {
    let recorder = unsafe { &mut *data.cast::<Recorder>() };

    if field.is_null() {
        recorder.fields.push(None);
        return;
    }

    let parser = unsafe { &mut *recorder.parser };
    if recorder.fields.is_empty() {
        recorder.saw_live_buffer = field.cast::<u8>() == parser.entry_buf;
        if len != 0 {
            unsafe { field.cast::<u8>().write(b'X') };
            recorder.first_field_mutated = unsafe { *parser.entry_buf == b'X' };
        }
    }

    let bytes = unsafe { std::slice::from_raw_parts(field.cast::<u8>(), len) };
    recorder.fields.push(Some(bytes.to_vec()));
}

unsafe extern "C" fn row_callback(term: c_int, data: *mut c_void) {
    let recorder = unsafe { &mut *data.cast::<Recorder>() };
    recorder.rows.push(term);
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn release_library() -> PathBuf {
    workspace_root().join("target/release/libcsv.so")
}

fn unique_output_path(stem: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{stem}-{}-{nonce}", process::id()))
}

fn compile_layout_probe() -> PathBuf {
    let output = unique_output_path("libcsv-layout-probe");
    let source = manifest_dir().join("tests/c/layout_probe.c");
    let status = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .arg("-std=c11")
        .arg("-I")
        .arg(workspace_root().join("original"))
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success(), "failed to compile layout probe");
    output
}

fn load_probe_layout() -> BTreeMap<String, usize> {
    let probe = compile_layout_probe();
    let output = Command::new(&probe).output().unwrap();
    assert!(output.status.success(), "layout probe failed");

    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let (name, value) = line.split_once('=').unwrap();
            (name.to_string(), value.parse::<usize>().unwrap())
        })
        .collect()
}

fn rust_layout() -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("size".into(), size_of::<csv_parser>()),
        ("align".into(), align_of::<csv_parser>()),
        ("pstate".into(), std::mem::offset_of!(csv_parser, pstate)),
        ("quoted".into(), std::mem::offset_of!(csv_parser, quoted)),
        ("spaces".into(), std::mem::offset_of!(csv_parser, spaces)),
        (
            "entry_buf".into(),
            std::mem::offset_of!(csv_parser, entry_buf),
        ),
        (
            "entry_pos".into(),
            std::mem::offset_of!(csv_parser, entry_pos),
        ),
        (
            "entry_size".into(),
            std::mem::offset_of!(csv_parser, entry_size),
        ),
        ("status".into(), std::mem::offset_of!(csv_parser, status)),
        ("options".into(), std::mem::offset_of!(csv_parser, options)),
        (
            "quote_char".into(),
            std::mem::offset_of!(csv_parser, quote_char),
        ),
        (
            "delim_char".into(),
            std::mem::offset_of!(csv_parser, delim_char),
        ),
        (
            "is_space".into(),
            std::mem::offset_of!(csv_parser, is_space),
        ),
        ("is_term".into(), std::mem::offset_of!(csv_parser, is_term)),
        (
            "blk_size".into(),
            std::mem::offset_of!(csv_parser, blk_size),
        ),
        (
            "malloc_func".into(),
            std::mem::offset_of!(csv_parser, malloc_func),
        ),
        (
            "realloc_func".into(),
            std::mem::offset_of!(csv_parser, realloc_func),
        ),
        (
            "free_func".into(),
            std::mem::offset_of!(csv_parser, free_func),
        ),
    ])
}

fn expected_exports() -> Vec<String> {
    fs::read_to_string(workspace_root().join("original/debian/libcsv3.symbols"))
        .unwrap()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with("libcsv.so.") {
                return None;
            }
            Some(trimmed.split_whitespace().next().unwrap().to_string())
        })
        .collect()
}

fn actual_exports(library: &Path) -> Vec<String> {
    let output = Command::new("readelf")
        .args(["--dyn-syms", "--wide"])
        .arg(library)
        .output()
        .unwrap();
    assert!(output.status.success(), "readelf --dyn-syms failed");

    let mut exports = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let columns: Vec<_> = line.split_whitespace().collect();
            if columns.len() < 8
                || columns[4] != "GLOBAL"
                || columns[5] != "DEFAULT"
                || columns[6] == "UND"
            {
                return None;
            }
            Some(columns[7].replace("@@", "@"))
        })
        .collect::<Vec<_>>();
    exports.sort();
    exports
}

#[test]
fn layout_matches_the_upstream_c_header() {
    assert_eq!(rust_layout(), load_probe_layout());
}

#[test]
fn soname_and_symbol_exports_match_debian_symbols() {
    let library = release_library();
    assert!(
        library.is_file(),
        "missing release library at {}",
        library.display()
    );

    let dynamic = Command::new("readelf")
        .args(["-d"])
        .arg(&library)
        .output()
        .unwrap();
    assert!(dynamic.status.success(), "readelf -d failed");
    let dynamic = String::from_utf8(dynamic.stdout).unwrap();
    assert!(
        dynamic.contains("Library soname: [libcsv.so.3]"),
        "unexpected dynamic section:\n{dynamic}",
    );

    let mut expected = expected_exports();
    expected.sort();
    let actual = actual_exports(&library);

    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 22);
    assert!(actual.iter().all(|name| name.ends_with("@Base")));
}

#[test]
fn direct_wrappers_preserve_upstream_behavior() {
    REALLOC_CALLS.store(0, Ordering::Relaxed);
    FREE_CALLS.store(0, Ordering::Relaxed);

    let mut parser = unsafe { std::mem::zeroed::<csv_parser>() };
    assert_eq!(unsafe { csv_init(&mut parser, CSV_APPEND_NULL) }, 0);
    assert!(parser.malloc_func.is_none());
    assert!(matches!(parser.realloc_func, Some(_)));
    assert!(matches!(parser.free_func, Some(_)));
    assert_eq!(
        unsafe { csv_get_opts(&mut parser) },
        c_int::from(CSV_APPEND_NULL)
    );
    assert_eq!(unsafe { csv_get_delim(&mut parser) }, b',');
    assert_eq!(unsafe { csv_get_quote(&mut parser) }, CSV_QUOTE);

    assert_eq!(unsafe { csv_set_opts(std::ptr::null_mut(), 0) }, -1);
    assert_eq!(unsafe { csv_get_opts(std::ptr::null_mut()) }, -1);
    assert_eq!(unsafe { csv_get_buffer_size(std::ptr::null_mut()) }, 0);

    unsafe { csv_set_blk_size(&mut parser, 2) };
    unsafe {
        csv_set_realloc_func(
            &mut parser,
            Some(realloc_hook as unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void),
        )
    };
    unsafe {
        csv_set_free_func(
            &mut parser,
            Some(free_hook as unsafe extern "C" fn(*mut c_void)),
        )
    };

    let mut recorder = Recorder {
        parser: &mut parser,
        ..Recorder::default()
    };

    let input = b"ab,cd";
    let consumed = unsafe {
        csv_parse(
            &mut parser,
            input.as_ptr().cast::<c_void>(),
            input.len(),
            Some(field_callback as unsafe extern "C" fn(*mut c_void, usize, *mut c_void)),
            Some(row_callback as unsafe extern "C" fn(c_int, *mut c_void)),
            (&mut recorder as *mut Recorder).cast::<c_void>(),
        )
    };
    assert_eq!(consumed, input.len());
    assert_eq!(
        unsafe {
            csv_fini(
                &mut parser,
                Some(field_callback as unsafe extern "C" fn(*mut c_void, usize, *mut c_void)),
                Some(row_callback as unsafe extern "C" fn(c_int, *mut c_void)),
                (&mut recorder as *mut Recorder).cast::<c_void>(),
            )
        },
        0,
    );

    assert_eq!(
        recorder.fields,
        vec![Some(b"Xb".to_vec()), Some(b"cd".to_vec())]
    );
    assert_eq!(recorder.rows, vec![END_OF_INPUT]);
    assert!(recorder.saw_live_buffer);
    assert!(recorder.first_field_mutated);
    assert!(unsafe { csv_get_buffer_size(&mut parser) } >= 4);
    assert!(REALLOC_CALLS.load(Ordering::Relaxed) >= 2);

    unsafe { csv_free(&mut parser) };
    assert_eq!(unsafe { csv_get_buffer_size(&mut parser) }, 0);
    assert_eq!(FREE_CALLS.load(Ordering::Relaxed), 1);

    let mut parser = unsafe { std::mem::zeroed::<csv_parser>() };
    assert_eq!(unsafe { csv_init(&mut parser, CSV_EMPTY_IS_NULL) }, 0);
    unsafe {
        csv_set_space_func(
            &mut parser,
            Some(underscore as unsafe extern "C" fn(c_uchar) -> c_int),
        );
        csv_set_term_func(
            &mut parser,
            Some(semicolon as unsafe extern "C" fn(c_uchar) -> c_int),
        );
        csv_set_delim(&mut parser, b',');
        csv_set_quote(&mut parser, b'"');
    }

    let mut custom = Recorder {
        parser: &mut parser,
        ..Recorder::default()
    };
    let custom_input = b"__,_,ab_;";
    assert_eq!(
        unsafe {
            csv_parse(
                &mut parser,
                custom_input.as_ptr().cast::<c_void>(),
                custom_input.len(),
                Some(field_callback as unsafe extern "C" fn(*mut c_void, usize, *mut c_void)),
                Some(row_callback as unsafe extern "C" fn(c_int, *mut c_void)),
                (&mut custom as *mut Recorder).cast::<c_void>(),
            )
        },
        custom_input.len(),
    );
    assert_eq!(
        unsafe {
            csv_fini(
                &mut parser,
                Some(field_callback as unsafe extern "C" fn(*mut c_void, usize, *mut c_void)),
                Some(row_callback as unsafe extern "C" fn(c_int, *mut c_void)),
                (&mut custom as *mut Recorder).cast::<c_void>(),
            )
        },
        0,
    );
    assert_eq!(
        custom.fields,
        vec![None, None, Some(b"ab".to_vec())],
        "custom predicates should match upstream empty-field behavior",
    );
    assert_eq!(custom.rows, vec![c_int::from(b';')]);
    unsafe { csv_free(&mut parser) };
}

#[test]
fn writer_wrappers_match_upstream_behavior() {
    let mut buffer = [0_u8; 4];
    let src = b"a\"b";
    let len = unsafe {
        csv_write(
            buffer.as_mut_ptr().cast::<c_void>(),
            buffer.len(),
            src.as_ptr().cast::<c_void>(),
            src.len(),
        )
    };
    assert_eq!(len, 6);
    assert_eq!(&buffer, b"\"a\"\"");

    let quoted = unsafe {
        csv_write2(
            std::ptr::null_mut(),
            0,
            src.as_ptr().cast::<c_void>(),
            src.len(),
            b'\'',
        )
    };
    assert_eq!(quoted, 5);

    let file = unsafe { tmpfile() };
    assert!(!file.is_null(), "tmpfile failed");
    let status = unsafe { csv_fwrite2(file, b"a'b".as_ptr().cast::<c_void>(), 3, b'\'') };
    assert_eq!(status, 0);
    unsafe { rewind(file) };

    let mut file_bytes = [0_u8; 16];
    let read = unsafe {
        fread(
            file_bytes.as_mut_ptr().cast::<c_void>(),
            1,
            file_bytes.len(),
            file,
        )
    };
    assert_eq!(&file_bytes[..read], b"'a''b'");
    assert_eq!(unsafe { fclose(file) }, 0);

    let success =
        unsafe { CStr::from_ptr(csv_strerror(c_int::from(CSV_SUCCESS)).cast::<c_char>()) };
    assert_eq!(success.to_bytes(), b"success");

    let parse = unsafe { CStr::from_ptr(csv_strerror(c_int::from(CSV_EPARSE)).cast::<c_char>()) };
    assert_eq!(
        parse.to_bytes(),
        b"error parsing data while strict checking enabled"
    );
}
