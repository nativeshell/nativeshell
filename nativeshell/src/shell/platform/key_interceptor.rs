use std::{
    cell::Cell,
    ffi::{c_void, CStr, CString},
};

use log::warn;

use super::key_event::process_key_event;

type SendPlatformMessage = extern "C" fn(usize, &Message) -> usize;

struct Message {
    size: usize,
    channel: *const i8,
    message: *const u8,
    message_size: usize,
    response_handle: isize,
}

extern "C" fn send_platform_message(engine: usize, message: &Message) -> usize {
    let channel = unsafe { CStr::from_ptr(message.channel) }
        .to_string_lossy()
        .to_string();
    if channel == "flutter/keyevent" {
        let channel = CString::new("nativeshell/keyevent").unwrap();
        let data = unsafe { std::slice::from_raw_parts(message.message, message.message_size) };
        let data: Vec<u8> = data.into();
        let data = process_key_event(data);
        let message = Message {
            size: message.size,
            channel: channel.as_ptr(),
            message: data.as_ptr(),
            message_size: data.len(),
            response_handle: message.response_handle,
        };
        SEND_PLATFORM_MESSAGE.with(|f| f.get().unwrap()(engine, &message))
    } else {
        SEND_PLATFORM_MESSAGE.with(|f| f.get().unwrap()(engine, &message))
    }
}

thread_local! {
    static SEND_PLATFORM_MESSAGE : Cell<Option<SendPlatformMessage>> = Cell::new(None);
}

#[repr(C)]
struct EngineProcTable {
    size: isize,
    create_aot_data: isize,
    collect_aot_data: isize,
    run: isize,
    shut_down: isize,
    inititalize: isize,
    deinitialize: isize,
    run_inititalized: isize,
    send_window_metric_event: isize,
    send_pointer_event: isize,
    send_key_event: isize,
    send_platform_message: SendPlatformMessage,
}

pub(super) fn override_key_event(proc_table: *mut c_void) {
    // Fragile as it may be, right now this seems to be the only reasonable way to intercept
    // keyboard events, which is absolutely required for menubar component

    let mut proc_table: &mut EngineProcTable = unsafe { std::mem::transmute(proc_table) };
    if proc_table.size != 280 {
        warn!(
            "Unexpected proc table size {}. Please update shell/platform/common/override_key_event",
            proc_table.size
        );
    }
    SEND_PLATFORM_MESSAGE.with(|v| {
        v.set(Some(proc_table.send_platform_message));
    });
    proc_table.send_platform_message = send_platform_message;
}
