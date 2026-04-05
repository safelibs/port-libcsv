#![cfg_attr(panic = "abort", no_std)]

extern crate alloc;
#[cfg(panic = "abort")]
use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
    mem,
    panic::PanicInfo,
    ptr,
};

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

#[cfg(panic = "abort")]
const MIN_ALIGN: usize = if cfg!(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm64ec",
    target_arch = "loongarch64",
    target_arch = "mips64",
    target_arch = "mips64r6",
    target_arch = "s390x",
    target_arch = "sparc64",
    target_arch = "riscv64",
    target_arch = "wasm64",
)) {
    16
} else {
    8
};

#[cfg(panic = "abort")]
extern "C" {
    fn abort() -> !;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn posix_memalign(out: *mut *mut c_void, align: usize, size: usize) -> i32;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

#[cfg(panic = "abort")]
struct LibcAllocator;

#[cfg(panic = "abort")]
unsafe impl GlobalAlloc for LibcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            unsafe { malloc(layout.size()) as *mut u8 }
        } else {
            unsafe { aligned_malloc(layout) }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            unsafe { calloc(layout.size(), 1) as *mut u8 }
        } else {
            let ptr = unsafe { self.alloc(layout) };
            if !ptr.is_null() {
                unsafe { ptr::write_bytes(ptr, 0, layout.size()) };
            }
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { free(ptr.cast::<c_void>()) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= new_size {
            unsafe { realloc(ptr.cast::<c_void>(), new_size) as *mut u8 }
        } else {
            unsafe { realloc_fallback(self, ptr, layout, new_size) }
        }
    }
}

#[cfg(panic = "abort")]
#[global_allocator]
static GLOBAL_ALLOCATOR: LibcAllocator = LibcAllocator;

#[cfg(panic = "abort")]
#[panic_handler]
fn panic_handler(_info: &PanicInfo<'_>) -> ! {
    unsafe { abort() }
}

#[cfg(all(panic = "abort", target_os = "linux", target_arch = "x86_64"))]
core::arch::global_asm!(
    r#"
    .hidden rust_eh_personality
    .globl rust_eh_personality
    .type rust_eh_personality,@function
rust_eh_personality:
    jmp abort@PLT
    .size rust_eh_personality, .-rust_eh_personality
    "#
);

#[cfg(panic = "abort")]
unsafe fn aligned_malloc(layout: Layout) -> *mut u8 {
    let mut out = ptr::null_mut();
    let align = layout.align().max(mem::size_of::<usize>());
    let result = unsafe { posix_memalign(&mut out, align, layout.size()) };
    if result == 0 {
        out.cast::<u8>()
    } else {
        ptr::null_mut()
    }
}

#[cfg(panic = "abort")]
unsafe fn realloc_fallback(
    allocator: &LibcAllocator,
    ptr: *mut u8,
    old_layout: Layout,
    new_size: usize,
) -> *mut u8 {
    let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, old_layout.align()) };
    let new_ptr = unsafe { allocator.alloc(new_layout) };

    if !new_ptr.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(ptr, new_ptr, old_layout.size().min(new_size));
            allocator.dealloc(ptr, old_layout);
        }
    }

    new_ptr
}

pub mod engine;
pub mod ffi;
pub mod rust_api;

pub use crate::engine::{strerror, Error};
pub use crate::rust_api::{
    write, write_to_buffer, write_to_buffer_with_quote, write_with_quote, Parser,
};

#[cfg(not(panic = "abort"))]
pub use crate::rust_api::{fwrite, fwrite_with_quote};
