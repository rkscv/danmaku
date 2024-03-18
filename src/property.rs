#![allow(clippy::uninit_assumed_init)]
#![allow(invalid_value)]

use crate::{
    ffi::{mpv_format, mpv_free, mpv_get_property},
    log_code, CTX,
};
use std::{
    ffi::{c_char, CStr, CString},
    mem::MaybeUninit,
    os::raw::c_void,
    ptr::addr_of_mut,
};

pub unsafe fn get_property_f64(name: &str) -> Option<f64> {
    let name = CString::new(name).unwrap();
    let mut data = MaybeUninit::<f64>::uninit().assume_init();
    let error = mpv_get_property(
        CTX,
        name.as_ptr(),
        mpv_format::MPV_FORMAT_DOUBLE,
        addr_of_mut!(data) as *mut c_void,
    );
    if error < 0 {
        log_code(error);
        None
    } else {
        Some(data)
    }
}

pub unsafe fn get_property_bool(name: &str) -> Option<bool> {
    let name = CString::new(name).unwrap();
    let mut data = MaybeUninit::<bool>::uninit().assume_init();
    let error = mpv_get_property(
        CTX,
        name.as_ptr(),
        mpv_format::MPV_FORMAT_FLAG,
        addr_of_mut!(data) as *mut c_void,
    );
    if error < 0 {
        log_code(error);
        None
    } else {
        Some(data)
    }
}

pub unsafe fn get_property_string(name: &str) -> Option<String> {
    let name = CString::new(name).unwrap();
    let mut data = MaybeUninit::<*mut c_char>::uninit().assume_init();
    let error = mpv_get_property(
        CTX,
        name.as_ptr(),
        mpv_format::MPV_FORMAT_STRING,
        addr_of_mut!(data) as *mut c_void,
    );
    if error < 0 {
        log_code(error);
        None
    } else {
        let value = CStr::from_ptr(data).to_str().unwrap().to_string();
        mpv_free(data as *mut c_void);
        Some(value)
    }
}
