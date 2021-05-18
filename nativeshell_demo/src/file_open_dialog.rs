#[allow(unused)]
use std::{
    cell::RefCell,
    mem::size_of,
    ptr::null_mut,
    rc::{Rc, Weak},
    time::Duration,
};

pub use nativeshell::{
    codec::{value::from_value, MethodCall, MethodCallReply, Value},
    shell::{Context, WindowHandle},
};

#[cfg(target_os = "linux")]
mod linux_imports {
    pub use gtk::{prelude::DialogExtManual, DialogExt, FileChooserDialogBuilder};
    pub use gtk::{FileChooserExt, GtkWindowExt};
}

#[cfg(target_os = "linux")]
use linux_imports::*;

#[cfg(target_os = "macos")]
mod mac_imports {
    pub use block::ConcreteBlock;
    pub use cocoa::{
        base::id,
        foundation::{NSArray, NSString, NSUInteger},
    };
    pub use objc::{
        msg_send,
        rc::{autoreleasepool, StrongPtr},
    };
}

#[cfg(target_os = "macos")]
use mac_imports::*;

#[cfg(target_os = "windows")]
mod win_imports {
    mod bindings {
        ::windows::include_bindings!();
    }

    pub use bindings::Windows::Win32::{SystemServices::*, WindowsAndMessaging::*};
    pub use widestring::WideStr;
}

#[cfg(target_os = "windows")]
use win_imports::*;

pub struct FileOpenDialogService {
    context: Rc<Context>,
    weak_self: RefCell<Weak<FileOpenDialogService>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileOpenRequest {
    parent_window: WindowHandle,
}

impl FileOpenDialogService {
    pub fn new(context: Rc<Context>) -> Rc<Self> {
        let res = Rc::new(Self {
            context: context.clone(),
            weak_self: RefCell::new(Default::default()),
        });
        *res.weak_self.borrow_mut() = Rc::downgrade(&res);
        res.initialize();
        res
    }

    fn initialize(&self) {
        let weak_self = self.weak_self.borrow().clone();
        self.context
            .message_manager
            .borrow_mut()
            .register_method_handler(
                "file_open_dialog_channel", //
                move |call, reply, _engine| {
                    if let Some(s) = weak_self.upgrade() {
                        s.on_method_call(call, reply);
                    }
                },
            );
    }

    fn on_method_call(&self, call: MethodCall<Value>, reply: MethodCallReply<Value>) {
        match call.method.as_str() {
            "showFileOpenDialog" => {
                let request: FileOpenRequest = from_value(&call.args).unwrap();
                self.open_file_dialog(request, reply);
            }
            _ => {
                reply.send_error("invalid_method", Some("Invalid method"), Value::Null);
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn open_file_dialog(&self, request: FileOpenRequest, reply: MethodCallReply<Value>) {
        let win = self
            .context
            .window_manager
            .borrow()
            .get_platform_window(request.parent_window);

        if let Some(win) = win {
            autoreleasepool(|| unsafe {
                let panel = StrongPtr::retain(msg_send![class!(NSOpenPanel), openPanel]);

                // We know that the callback will be called only once, but rust doesn't;
                let reply = RefCell::new(Some(reply));

                let panel_copy = panel.clone();
                let cb = move |response: NSUInteger| {
                    let reply = reply.take();
                    if let Some(reply) = reply {
                        if response == 1 {
                            let urls: id = msg_send![*panel_copy, URLs];
                            if NSArray::count(urls) > 0 {
                                let url = NSArray::objectAtIndex(urls, 0);
                                let string: id = msg_send![url, absoluteString];
                                let path = Self::from_nsstring(string);
                                reply.send_ok(Value::String(path));
                                return;
                            }
                        }
                        reply.send_ok(Value::Null);
                    }
                };

                let handler = ConcreteBlock::new(cb).copy();
                let () =
                    msg_send![*panel, beginSheetModalForWindow: win completionHandler:&*handler];
            });
        } else {
            reply.send_error("no_window", Some("Platform window not found"), Value::Null);
        }
    }

    #[cfg(target_os = "macos")]
    fn from_nsstring(ns_string: id) -> String {
        use std::os::raw::c_char;
        use std::slice;
        unsafe {
            let bytes: *const c_char = msg_send![ns_string, UTF8String];
            let bytes = bytes as *const u8;
            let len = NSString::len(ns_string);
            let bytes = slice::from_raw_parts(bytes, len);
            std::str::from_utf8(bytes).unwrap().into()
        }
    }

    #[cfg(target_os = "windows")]
    fn open_file_dialog(&self, request: FileOpenRequest, reply: MethodCallReply<Value>) {
        let win = self
            .context
            .window_manager
            .borrow()
            .get_platform_window(request.parent_window);

        if let Some(win) = win {
            let cb = move || {
                let mut file = Vec::<u16>::new();
                file.resize(4096, 0);

                let mut ofn = OPENFILENAMEW {
                    lStructSize: size_of::<OPENFILENAMEW>() as u32,
                    hwndOwner: HWND(win.0),
                    hInstance: HINSTANCE(0),
                    lpstrFilter: PWSTR::default(),
                    lpstrCustomFilter: PWSTR::default(),
                    nMaxCustFilter: 0,
                    nFilterIndex: 0,
                    lpstrFile: PWSTR(file.as_mut_ptr()),
                    nMaxFile: file.len() as u32,
                    lpstrFileTitle: PWSTR::default(),
                    nMaxFileTitle: 0,
                    lpstrInitialDir: PWSTR::default(),
                    lpstrTitle: PWSTR::default(),
                    Flags: OPEN_FILENAME_FLAGS(0),
                    nFileOffset: 0,
                    nFileExtension: 0,
                    lpstrDefExt: PWSTR::default(),
                    lCustData: LPARAM(0),
                    lpfnHook: None,
                    lpTemplateName: PWSTR::default(),
                    pvReserved: null_mut(),
                    dwReserved: 0,
                    FlagsEx: OPEN_FILENAME_FLAGS_EX(0),
                };

                let res = unsafe { GetOpenFileNameW(&mut ofn as *mut _) == TRUE };
                if !res {
                    reply.send_ok(Value::Null);
                } else {
                    let name = WideStr::from_slice(&file).to_string_lossy();
                    reply.send_ok(Value::String(name));
                }
            };
            self.context
                .run_loop
                .borrow()
                .schedule(cb, Duration::from_secs(0))
                .detach();
        } else {
            reply.send_error("no_window", Some("Platform window not found"), Value::Null);
        }
    }

    #[cfg(target_os = "linux")]
    fn open_file_dialog(&self, request: FileOpenRequest, reply: MethodCallReply<Value>) {
        let win = self
            .context
            .window_manager
            .borrow()
            .get_platform_window(request.parent_window);

        if let Some(win) = win {
            let dialog = FileChooserDialogBuilder::new()
                .transient_for(&win)
                .modal(true)
                .action(gtk::FileChooserAction::Open)
                .build();

            dialog.add_buttons(&[
                ("Open", gtk::ResponseType::Ok),
                ("Cancel", gtk::ResponseType::Cancel),
            ]);

            // Platform messages will be processed while dialog is running so
            // make sure it is cheduled on next run loop turn
            self.context
                .run_loop
                .borrow()
                .schedule_next(move || {
                    let res = dialog.run();
                    let res = match res {
                        gtk::ResponseType::Ok => {
                            let path = dialog.get_filename();
                            path.map(|path| Value::String(path.to_string_lossy().into()))
                                .unwrap_or_default()
                        }
                        _ => Value::Null,
                    };
                    dialog.close();
                    reply.send(Ok(res));
                })
                .detach();
        }
    }
}

impl Drop for FileOpenDialogService {
    fn drop(&mut self) {
        self.context
            .message_manager
            .borrow_mut()
            .unregister_message_handler("file_open_dialog_channel");
    }
}
