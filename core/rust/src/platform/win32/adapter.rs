use super::sys::windows::*;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

thread_local! {
    static WINDOW_CLASS: RefCell<Weak<WindowClass>> = RefCell::new(Weak::new());
}

struct WindowClass {
    pub class_name: String,
}

fn to_utf16(s: &str) -> Vec<u16> {
    let mut string: Vec<u16> = s.encode_utf16().collect();
    string.push(0);
    string
}

impl WindowClass {
    pub fn get() -> Rc<Self> {
        WINDOW_CLASS.with(|window_class| {
            let res = window_class.borrow().upgrade();
            match res {
                Some(class) => class,
                None => {
                    let res = Rc::new(Self::new());
                    window_class.replace(Rc::downgrade(&res));
                    res
                }
            }
        })
    }

    fn new() -> Self {
        let res = WindowClass {
            class_name: "NATIVESHELL_WINDOW".into(),
        };
        res.register();
        res
    }

    fn register(&self) {
        unsafe {
            let mut class_name = to_utf16(&self.class_name);
            let class = WNDCLASSW {
                style: 0,
                lpfnWndProc: wnd_proc,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(std::ptr::null_mut()),
                hIcon: 0,
                hCursor: LoadCursorW(0, IDC_ARROW),
                hbrBackground: 0,
                lpszMenuName: std::ptr::null_mut(),
                lpszClassName: class_name.as_mut_ptr(),
            };
            RegisterClassW(&class as *const _);
        }
    }

    fn unregister(&mut self) {
        unsafe {
            UnregisterClassW(to_utf16(&self.class_name).as_mut_ptr(), 0);
        }
    }
}

unsafe extern "system" fn wnd_proc(
    h_wnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create_struct = &*(l_param as *const CREATESTRUCTW);
        SetWindowLongPtrW(h_wnd, GWLP_USERDATA, create_struct.lpCreateParams as isize);
    }

    let ptr = GetWindowLongPtrW(h_wnd, GWLP_USERDATA);
    if ptr != 0 {
        let bridge = &*(ptr as *const EventBridge);
        let handler = &*(bridge.handler);
        let res = handler.wnd_proc(h_wnd, msg, w_param, l_param);
        if msg == WM_NCDESTROY as u32 {
            // make sure bridge is dropped
            Box::<EventBridge>::from_raw(ptr as *mut EventBridge);
        }
        return res;
    }

    DefWindowProcW(h_wnd, msg, w_param, l_param)
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        self.unregister();
    }
}

struct EventBridge {
    handler: *const dyn WindowAdapter,
    _class: Rc<WindowClass>, // keep class alive
}

pub trait WindowAdapter {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;

    fn create_window(&self, title: &str, style: WINDOW_STYLE, ex_style: WINDOW_STYLE) -> HWND
    where
        Self: Sized,
    {
        unsafe {
            let s = self as &dyn WindowAdapter;
            let class = WindowClass::get();
            let ptr = std::mem::transmute(s);
            let bridge = Box::new(EventBridge {
                handler: ptr,
                _class: class.clone(),
            });

            let mut class_name = to_utf16(&class.class_name);
            let mut title = to_utf16(title);
            CreateWindowExW(
                ex_style,
                class_name.as_mut_ptr(),
                title.as_mut_ptr(),
                style,
                100,
                100,
                200,
                200,
                0,
                0,
                GetModuleHandleW(std::ptr::null_mut()),
                Box::into_raw(bridge) as *mut _,
            )
        }
    }
}
