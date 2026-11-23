//! Minimal MSVC CRT shims required to link Microsoft's `WebView2LoaderStatic.lib`
//! with the GNU (MinGW) toolchain.
//!
//! The static loader library is compiled by MSVC and references a handful of
//! MSVC CRT internals (stack-cookie helpers and the "magic static" guard
//! helpers). MinGW provides equivalents for its own runtime, but the symbol
//! names differ, so we supply compatible implementations here.

#![allow(non_upper_case_globals, non_snake_case)]

use core::sync::atomic::{AtomicI32, Ordering};

/// MSVC stack cookie (default x64 value used by the MSVC CRT).
#[no_mangle]
pub static mut __security_cookie: usize = 0x0000_2B99_2DDF_A232;

/// MSVC stack-cookie verification hook. The GNU runtime guards the stack on
/// its own, so an empty body is sufficient here.
#[no_mangle]
pub extern "C" fn __security_check_cookie(_cookie: usize) {}

/// Thread-local epoch used by MSVC's thread-safe static initialisation.
#[no_mangle]
pub static mut _Init_thread_epoch: i32 = 0;

/// `std::nothrow_t` instance referenced by the C++ loader code.
#[export_name = "?nothrow@std@@3Unothrow_t@1@B"]
pub static NOTHROW_TAG: u8 = 0;

// The loader's C++ code pairs `new` with `delete`, so the CRT heap allocator
// is the correct and layout-independent choice here.
extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

/// `operator new(size_t, std::nothrow_t const&)`
#[export_name = "??2@YAPEAX_KAEBUnothrow_t@std@@@Z"]
pub extern "C" fn operator_new_nothrow(size: usize) -> *mut u8 {
    // SAFETY: forwarding to the CRT allocator with a normalised size.
    unsafe { malloc(size.max(1)) }
}

/// `operator new[](size_t)`
#[export_name = "??_U@YAPEAX_K@Z"]
pub extern "C" fn operator_new_array(size: usize) -> *mut u8 {
    // SAFETY: forwarding to the CRT allocator with a normalised size.
    unsafe { malloc(size.max(1)) }
}

/// `operator delete(void*, unsigned long long)`
#[export_name = "??3@YAXPEAX_K@Z"]
pub unsafe extern "C" fn operator_delete_sized(ptr: *mut u8, _size: usize) {
    if !ptr.is_null() {
        free(ptr);
    }
}

/// `operator delete(void*)`
#[export_name = "??3@YAXPEAX@Z"]
pub unsafe extern "C" fn operator_delete(ptr: *mut u8) {
    if !ptr.is_null() {
        free(ptr);
    }
}

/// `operator delete[](void*)`
#[export_name = "??_V@YAXPEAX@Z"]
pub unsafe extern "C" fn operator_delete_array(ptr: *mut u8) {
    if !ptr.is_null() {
        free(ptr);
    }
}

fn guard(ptr: *mut i32) -> &'static AtomicI32 {
    // SAFETY: the guard variable is an `int` emitted by the MSVC compiler in
    // static storage; reading and writing it atomically is well defined.
    unsafe { &*(ptr as *const AtomicI32) }
}

/// Claims the initialisation of a function-local static (MSVC ABI).
#[no_mangle]
pub unsafe extern "C" fn _Init_thread_header(ptr: *mut i32) {
    if ptr.is_null() {
        return;
    }
    let slot = guard(ptr);
    loop {
        match slot.compare_exchange(0, -1, Ordering::Acquire, Ordering::Acquire) {
            Ok(_) => return,
            Err(value) if value == 1 => return,
            Err(_) => std::thread::yield_now(),
        }
    }
}

/// Marks the initialisation of a function-local static as complete (MSVC ABI).
#[no_mangle]
pub unsafe extern "C" fn _Init_thread_footer(ptr: *mut i32) {
    if ptr.is_null() {
        return;
    }
    guard(ptr).store(1, Ordering::Release);
}
