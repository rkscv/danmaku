use crate::{
    ffi::{mpv_command_node, mpv_format, mpv_node, mpv_node_list, u},
    log_code, CTX,
};
use std::{
    ffi::CString,
    ptr::{addr_of_mut, null_mut},
};

pub unsafe fn osd_overlay(data: &str, width: i64, height: i64) {
    let mut keys = ["name", "id", "format", "data", "res_x", "res_y"]
        .map(|key| CString::new(key).unwrap().into_raw());
    let value1 = CString::new("osd-overlay").unwrap().into_raw();
    let value3 = CString::new("ass-events").unwrap().into_raw();
    let value4 = CString::new(data).unwrap().into_raw();
    let mut values = [
        mpv_node {
            format: mpv_format::MPV_FORMAT_STRING,
            u: u { string: value1 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_INT64,
            u: u { int64: 0 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_STRING,
            u: u { string: value3 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_STRING,
            u: u { string: value4 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_INT64,
            u: u { int64: width },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_INT64,
            u: u { int64: height },
        },
    ];
    assert_eq!(keys.len(), values.len());

    let mut list = mpv_node_list {
        num: keys.len().try_into().unwrap(),
        values: values.as_mut_ptr(),
        keys: keys.as_mut_ptr(),
    };
    let mut args = mpv_node {
        format: mpv_format::MPV_FORMAT_NODE_MAP,
        u: u {
            list: addr_of_mut!(list),
        },
    };
    let error = mpv_command_node(CTX, addr_of_mut!(args), null_mut());
    if error < 0 {
        log_code(error);
    }

    _ = keys.map(|key| CString::from_raw(key));
    _ = CString::from_raw(value1);
    _ = CString::from_raw(value3);
    _ = CString::from_raw(value4);
}

pub unsafe fn remove_overlay() {
    let mut keys =
        ["name", "id", "format", "data"].map(|key| CString::new(key).unwrap().into_raw());
    let value1 = CString::new("osd-overlay").unwrap().into_raw();
    let value3 = CString::new("none").unwrap().into_raw();
    let value4 = CString::new("").unwrap().into_raw();
    let mut values = [
        mpv_node {
            format: mpv_format::MPV_FORMAT_STRING,
            u: u { string: value1 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_INT64,
            u: u { int64: 0 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_STRING,
            u: u { string: value3 },
        },
        mpv_node {
            format: mpv_format::MPV_FORMAT_STRING,
            u: u { string: value4 },
        },
    ];
    assert_eq!(keys.len(), values.len());

    let mut list = mpv_node_list {
        num: keys.len().try_into().unwrap(),
        values: values.as_mut_ptr(),
        keys: keys.as_mut_ptr(),
    };
    let mut args = mpv_node {
        format: mpv_format::MPV_FORMAT_NODE_MAP,
        u: u {
            list: addr_of_mut!(list),
        },
    };
    let error = mpv_command_node(CTX, addr_of_mut!(args), null_mut());
    if error < 0 {
        log_code(error);
    }

    _ = keys.map(|key| CString::from_raw(key));
    _ = CString::from_raw(value1);
    _ = CString::from_raw(value3);
    _ = CString::from_raw(value4);
}
