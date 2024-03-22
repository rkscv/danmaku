#![allow(clippy::missing_safety_doc)]

pub mod danmaku;
pub mod ffi;
pub mod log;
pub mod options;
pub mod overlay;
pub mod property;

use crate::{
    danmaku::{get_danmaku, Danmaku},
    ffi::{
        mpv_client_name, mpv_command, mpv_event_client_message, mpv_event_id, mpv_format,
        mpv_handle, mpv_observe_property, mpv_wait_event,
    },
    log::{log_code, log_error},
    options::read_options,
    overlay::{osd_overlay, remove_overlay},
    property::{get_property_bool, get_property_f64, get_property_string},
};
use std::{
    cmp::max,
    ffi::{CStr, CString},
    os::raw::c_int,
    ptr::{null, null_mut},
    slice::from_raw_parts,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{runtime::Builder, spawn, sync::Mutex};

const DURATION: f64 = 12.;
const INTERVAL: f64 = 0.005;

pub static mut CTX: *mut mpv_handle = null_mut();
pub static mut CLIENT_NAME: &str = "";
pub static mut FONT_SIZE: f64 = 40.;

#[no_mangle]
unsafe extern "C" fn mpv_open_cplugin(ctx: *mut mpv_handle) -> c_int {
    CTX = ctx;
    CLIENT_NAME = CStr::from_ptr(mpv_client_name(ctx)).to_str().unwrap();
    read_options()
        .map_err(|e| log_error(e))
        .ok()
        .flatten()
        .unwrap_or_default()
        .get("font_size")
        .and_then(|s| s.parse().ok().filter(|&s| s > 0.))
        .inspect(|&s| FONT_SIZE = s);

    Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(main(ctx))
}

async unsafe fn main(ctx: *mut mpv_handle) -> c_int {
    let error = mpv_observe_property(ctx, 0, c"pause".as_ptr(), mpv_format::MPV_FORMAT_NONE);
    if error < 0 {
        log_code(error);
        return -1;
    }

    let comments = Arc::new(Mutex::new(None));
    let enabled = Arc::new(AtomicBool::new(false));
    let mut handle = spawn(async {});
    loop {
        let timeout = if enabled.load(Ordering::SeqCst)
            && matches!(get_property_bool(c"pause"), Some(false))
        {
            INTERVAL
        } else {
            -1.
        };
        let event = &*mpv_wait_event(ctx, timeout);
        match event.event_id {
            mpv_event_id::MPV_EVENT_SHUTDOWN => {
                handle.abort();
                return 0;
            }
            mpv_event_id::MPV_EVENT_FILE_LOADED => {
                handle.abort();
                *comments.lock().await = None;
                if enabled.load(Ordering::SeqCst) {
                    remove_overlay();
                    handle = spawn(get(comments.clone(), enabled.clone()));
                }
            }
            mpv_event_id::MPV_EVENT_SEEK => {
                if enabled.load(Ordering::SeqCst) {
                    if let Some(comments) = &mut *comments.lock().await {
                        reset(comments);
                    }
                }
            }
            mpv_event_id::MPV_EVENT_CLIENT_MESSAGE => {
                let data = &*(event.data as *mut mpv_event_client_message);
                if !from_raw_parts(data.args, data.num_args.try_into().unwrap())
                    .first()
                    .map(|&arg| CStr::from_ptr(arg) == c"toggle-danmaku")
                    .unwrap_or_default()
                {
                    continue;
                }
                if enabled.fetch_xor(true, Ordering::SeqCst) {
                    remove_overlay();
                    osd_message("Danmaku: off");
                } else {
                    match &mut *comments.lock().await {
                        Some(comments) => {
                            reset(comments);
                            loaded(comments.len());
                        }
                        None => {
                            osd_message("Danmaku: on");
                            handle.abort();
                            handle = spawn(get(comments.clone(), enabled.clone()));
                        }
                    }
                }
            }
            _ => (),
        }

        if enabled.load(Ordering::SeqCst) {
            if let Some(comments) = &mut *comments.lock().await {
                render(comments);
            }
        }
    }
}

unsafe fn render(comments: &mut Vec<Danmaku>) -> Option<()> {
    let width = get_property_f64(c"osd-width").filter(|&w| w > 0.)?;
    let height = get_property_f64(c"osd-height").filter(|&h| h > 0.)?;
    let pos = get_property_f64(c"time-pos")?;
    let speed = get_property_f64(c"speed")?;
    let spacing = FONT_SIZE / 10.;
    let mut ends = Vec::new();
    ends.resize(max((height / (FONT_SIZE + spacing)) as usize, 1), None);

    let mut danmaku = Vec::new();
    for comment in comments {
        if comment.time > pos + DURATION / 2. {
            break;
        }

        let x = comment
            .x
            .get_or_insert_with(|| width - (pos - comment.time) * width / DURATION);
        if *x + comment.count as f64 * FONT_SIZE + spacing < 0. {
            continue;
        }
        let row = *comment.row.get_or_insert_with(|| {
            ends.iter()
                .enumerate()
                .find(|(_, end)| end.map(|end: f64| end < *x).unwrap_or(true))
                .map(|(row, _)| row)
                .unwrap_or_else(|| {
                    ends.iter()
                        .enumerate()
                        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .map(|(row, _)| row)
                        .unwrap()
                })
        });
        danmaku.push(format!(
            "{{\\pos({},{})\\c&H{:x}{:x}{:x}&\\alpha&H30\\fs{}\\bord1.5\\b1\\q2}}{}",
            *x,
            row as f64 * (FONT_SIZE + spacing),
            comment.b,
            comment.g,
            comment.r,
            FONT_SIZE,
            comment.message
        ));

        *x -= width / DURATION * speed * INTERVAL;
        if let Some(end) = ends.get_mut(row) {
            let new_end = *x + comment.count as f64 * FONT_SIZE + spacing;
            match end {
                Some(end) => *end = end.max(new_end),
                None => *end = Some(new_end),
            }
        }
    }
    osd_overlay(&danmaku.join("\n"), width as i64, height as i64);
    Some(())
}

async unsafe fn get(comments: Arc<Mutex<Option<Vec<Danmaku>>>>, enabled: Arc<AtomicBool>) {
    let Some(path) = get_property_string(c"path") else {
        return;
    };
    match get_danmaku(path).await {
        Ok(mut danmaku) => {
            if enabled.load(Ordering::SeqCst) {
                if let Some(true) = get_property_bool(c"pause") {
                    render(&mut danmaku);
                }
                loaded(danmaku.len());
            }
            *comments.lock().await = Some(danmaku)
        }
        Err(error) => {
            if enabled.load(Ordering::SeqCst) {
                osd_message(&format!("Danmaku: {}", error));
            }
            log_error(error);
        }
    }
}

fn reset(comments: &mut Vec<Danmaku>) {
    for comment in comments {
        comment.x = None;
        comment.row = None;
    }
}

unsafe fn loaded(n: usize) {
    osd_message(&format!(
        "Loaded {} danmaku comment{}",
        n,
        if n > 1 { "s" } else { "" }
    ));
}

unsafe fn osd_message(text: &str) {
    let arg2 = CString::new(text).unwrap();
    let mut args = [c"show-text".as_ptr(), arg2.as_ptr(), null()];
    let error = mpv_command(CTX, args.as_mut_ptr());
    if error < 0 {
        log_code(error);
    }
}
