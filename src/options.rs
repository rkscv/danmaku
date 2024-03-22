use crate::{
    ffi::{mpv_command_ret, mpv_error_string, mpv_format, mpv_free_node_contents, mpv_node},
    CLIENT_NAME, CTX,
};
use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fs::File,
    io::{BufRead, BufReader, ErrorKind},
    mem::MaybeUninit,
    ptr::{addr_of_mut, null},
};

#[allow(clippy::uninit_assumed_init)]
#[allow(invalid_value)]
pub unsafe fn read_options() -> Result<Option<HashMap<String, String>>> {
    let arg2 = CString::new(format!("~~/script-opts/{}.conf", CLIENT_NAME)).unwrap();
    let mut args = [c"expand-path".as_ptr(), arg2.as_ptr(), null()];
    let mut result = MaybeUninit::<mpv_node>::uninit().assume_init();
    let error = mpv_command_ret(CTX, args.as_mut_ptr(), addr_of_mut!(result));
    if error < 0 {
        return Err(anyhow!(
            "{}",
            CStr::from_ptr(mpv_error_string(error)).to_str().unwrap()
        ));
    }
    assert_eq!(result.format, mpv_format::MPV_FORMAT_STRING);
    let path = CStr::from_ptr(result.u.string)
        .to_str()
        .unwrap()
        .to_string();
    mpv_free_node_contents(addr_of_mut!(result));

    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut opts = HashMap::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if !line.starts_with('#') {
            if let Some((k, v)) = line.split_once('=') {
                opts.insert(k.into(), v.into());
            }
        }
    }
    Ok(Some(opts))
}
