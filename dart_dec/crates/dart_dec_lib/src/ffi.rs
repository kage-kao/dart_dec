use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use crate::DartDecContext;

/// Open a Dart AOT binary and return a context handle
#[no_mangle]
pub extern "C" fn dart_dec_open(path: *const c_char) -> *mut DartDecContext {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match DartDecContext::open(Path::new(path_str)) {
        Ok(mut ctx) => {
            // Also parse the snapshot
            let _ = ctx.parse_snapshot();
            Box::into_raw(Box::new(ctx))
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get all classes as a JSON string
#[no_mangle]
pub extern "C" fn dart_dec_get_classes_json(ctx: *mut DartDecContext) -> *mut c_char {
    if ctx.is_null() {
        return std::ptr::null_mut();
    }

    let ctx = unsafe { &*ctx };
    match ctx.get_classes_json() {
        Ok(json) => match CString::new(json) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Decompile a specific function
#[no_mangle]
pub extern "C" fn dart_dec_decompile_function(
    ctx: *mut DartDecContext,
    class_name: *const c_char,
    func_name: *const c_char,
) -> *mut c_char {
    if ctx.is_null() || class_name.is_null() || func_name.is_null() {
        return std::ptr::null_mut();
    }

    let ctx = unsafe { &*ctx };
    let class_str = unsafe { CStr::from_ptr(class_name) }
        .to_str()
        .unwrap_or("");
    let func_str = unsafe { CStr::from_ptr(func_name) }
        .to_str()
        .unwrap_or("");

    match ctx.decompile_function(class_str, func_str) {
        Ok(code) => match CString::new(code) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get all strings as JSON array
#[no_mangle]
pub extern "C" fn dart_dec_get_strings_json(ctx: *mut DartDecContext) -> *mut c_char {
    if ctx.is_null() {
        return std::ptr::null_mut();
    }

    let ctx = unsafe { &*ctx };
    match ctx.get_strings() {
        Ok(strings) => {
            let json = serde_json::to_string(&strings).unwrap_or_default();
            match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string allocated by dart_dec
#[no_mangle]
pub extern "C" fn dart_dec_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Close and free a DartDecContext
#[no_mangle]
pub extern "C" fn dart_dec_close(ctx: *mut DartDecContext) {
    if !ctx.is_null() {
        unsafe {
            let _ = Box::from_raw(ctx);
        }
    }
}
