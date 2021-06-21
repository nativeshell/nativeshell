use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    rc::Rc,
    slice,
};

use crate::shell::BinaryMessengerReply;

use super::{
    error::{PlatformError, PlatformResult},
    flutter_sys::{
        size_t, FlutterDesktopMessage, FlutterDesktopMessengerRef, FlutterDesktopMessengerSend,
        FlutterDesktopMessengerSendResponse, FlutterDesktopMessengerSendWithReply,
        FlutterDesktopMessengerSetCallback,
    },
};

type Callback = Box<dyn Fn(&[u8], BinaryMessengerReply)>;

pub struct PlatformBinaryMessenger {
    handle: Rc<FlutterDesktopMessengerRef>,
    callbacks: RefCell<HashMap<String, Rc<Callback>>>,
    active_callbacks: RefCell<HashSet<String>>,
}

impl PlatformBinaryMessenger {
    pub fn from_handle(handle: FlutterDesktopMessengerRef) -> Self {
        Self {
            handle: Rc::new(handle),
            callbacks: RefCell::new(HashMap::new()),
            active_callbacks: RefCell::new(HashSet::new()),
        }
    }

    unsafe extern "C" fn message_callback(
        _messenger: FlutterDesktopMessengerRef,
        message: *const FlutterDesktopMessage,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let s = user_data as *const Self;
        let messenger_impl = &*s;

        let message = &*message;
        let channel = CStr::from_ptr(message.channel).to_string_lossy();
        let callback: Option<Rc<Callback>> = {
            messenger_impl
                .callbacks
                .borrow_mut()
                .get(channel.as_ref())
                .cloned()
        };

        let messenger_weak = Rc::downgrade(&messenger_impl.handle);

        if let Some(callback) = callback {
            let data = slice::from_raw_parts(message.message, message.message_size);

            let response_handle = message.response_handle;
            callback(
                data,
                BinaryMessengerReply::new(move |data| {
                    if let Some(messenger) = messenger_weak.upgrade() {
                        FlutterDesktopMessengerSendResponse(
                            *messenger,
                            response_handle,
                            data.as_ptr(),
                            data.len(),
                        );
                    }
                }),
            );
        }
    }

    pub fn register_channel_handler<F>(&self, channel: &str, callback: F)
    where
        F: Fn(&[u8], BinaryMessengerReply) + 'static,
    {
        let callback = Box::new(callback);
        self.callbacks
            .borrow_mut()
            .insert(channel.into(), Rc::new(callback));
        self.active_callbacks.borrow_mut().insert(channel.into());
        let channel = CString::new(channel).unwrap();
        unsafe {
            let self_ptr = self as *const Self as *mut c_void;
            FlutterDesktopMessengerSetCallback(
                *self.handle,
                channel.as_ptr(),
                Some(Self::message_callback),
                self_ptr,
            );
        }
    }

    pub fn unregister_channel_handler(&self, channel: &str) {
        self.callbacks.borrow_mut().remove(channel);
        self.active_callbacks.borrow_mut().remove(channel);

        let channel = CString::new(channel).unwrap();
        unsafe {
            FlutterDesktopMessengerSetCallback(
                *self.handle,
                channel.as_ptr(),
                None,
                std::ptr::null_mut(),
            );
        }
    }

    unsafe extern "C" fn send_message_reply(
        data: *const u8,
        data_size: size_t,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let data = slice::from_raw_parts(data, data_size);
        let b: Box<Box<dyn FnOnce(&[u8])>> = Box::from_raw(user_data as *mut _);
        b(data);
    }

    pub fn send_message<F>(&self, channel: &str, message: &[u8], reply: F) -> PlatformResult<()>
    where
        F: FnOnce(&[u8]) + 'static,
    {
        let b: Box<dyn FnOnce(&[u8])> = Box::new(reply);
        let b = Box::new(b);
        let c_channel = CString::new(channel).unwrap();
        if !unsafe {
            FlutterDesktopMessengerSendWithReply(
                *self.handle,
                c_channel.as_ptr(),
                message.as_ptr(),
                message.len(),
                Some(Self::send_message_reply),
                Box::into_raw(b) as *mut c_void,
            )
        } {
            Err(PlatformError::SendMessageFailure {
                channel: channel.into(),
            })
        } else {
            Ok(())
        }
    }

    pub fn post_message(&self, channel: &str, message: &[u8]) -> PlatformResult<()> {
        let c_channel = CString::new(channel).unwrap();
        if !unsafe {
            FlutterDesktopMessengerSend(
                *self.handle,
                c_channel.as_ptr(),
                message.as_ptr(),
                message.len(),
            )
        } {
            Err(PlatformError::SendMessageFailure {
                channel: channel.into(),
            })
        } else {
            Ok(())
        }
    }
}

impl Drop for PlatformBinaryMessenger {
    fn drop(&mut self) {
        let callbacks = self.active_callbacks.borrow().clone();
        for channel in callbacks {
            self.unregister_channel_handler(&channel);
        }
    }
}
