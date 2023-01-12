use windows::{
    core::{IntoParam, Param},
    Win32::{
        Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, WPARAM},
        Graphics::Gdi::HBRUSH,
        System::LibraryLoader::{FreeLibrary, GetModuleHandleW, GetProcAddress, LoadLibraryW},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, LoadCursorW, RegisterClassW, UnregisterClassW,
            CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HMENU, IDC_ARROW,
            WINDOW_EX_STYLE, WINDOW_STYLE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW, WS_DLGFRAME,
            WS_EX_APPWINDOW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW, WS_SYSMENU,
            WS_THICKFRAME,
        },
    },
};

use super::util::direct_composition_supported;
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
        let mut res = WindowClass {
            class_name: "nativeshell_FLUTTER_WINDOW".into(),
        };
        res.register();
        res
    }

    fn register(&mut self) {
        unsafe {
            let class_name: Param<PWSTR> = self.class_name.clone().into_param();
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: HINSTANCE(GetModuleHandleW(PWSTR::default()).0),
                hIcon: Default::default(),
                hCursor: LoadCursorW(HINSTANCE(0), IDC_ARROW),
                hbrBackground: HBRUSH(0),
                lpszMenuName: PWSTR::default(),
                lpszClassName: class_name.abi(),
            };
            RegisterClassW(&class);
        }
    }

    fn unregister(&mut self) {
        unsafe {
            UnregisterClassW(self.class_name.as_str(), HINSTANCE(0));
        }
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        self.unregister();
    }
}

// Adapter for handling window message in rust object
pub trait WindowAdapter {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;

    fn default_wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(h_wnd, msg, w_param, l_param) }
    }

    fn create_window(&self, title: &str) -> HWND
    where
        Self: Sized,
    {
        let mut ex_flags = WS_EX_APPWINDOW;
        if direct_composition_supported() {
            ex_flags |= WS_EX_NOREDIRECTIONBITMAP;
        }

        self.create_window_custom(
            title,
            WS_OVERLAPPEDWINDOW | WS_THICKFRAME | WS_SYSMENU | WS_DLGFRAME,
            ex_flags,
        )
    }

    fn create_window_custom(
        &self,
        title: &str,
        style: WINDOW_STYLE,
        ex_style: WINDOW_EX_STYLE,
    ) -> HWND
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

            let res = CreateWindowExW(
                ex_style,
                class.class_name.as_str(),
                title,
                style,
                100,
                100,
                200,
                200,
                HWND(0),
                HMENU(0),
                HINSTANCE(GetModuleHandleW(PWSTR::default()).0),
                Box::into_raw(bridge) as *mut _,
            );
            res
        }
    }
}

struct EventBridge {
    handler: *const dyn WindowAdapter,
    _class: Rc<WindowClass>, // keep class alive
}

// Missing from metadata for now
#[link(name = "USER32")]
extern "system" {
    pub fn SetWindowLongPtrW(h_wnd: HWND, n_index: i32, dw_new_long: isize) -> isize;
    pub fn GetWindowLongPtrW(h_wnd: HWND, n_index: i32) -> isize;
}

extern "system" fn wnd_proc(h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        #[allow(clippy::single_match)]
        match msg {
            WM_NCCREATE => {
                let create_struct = &*(l_param.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(
                    h_wnd,
                    GWLP_USERDATA.0,
                    create_struct.lpCreateParams as isize,
                );
                enable_full_dpi_support(h_wnd);
            }
            _ => {}
        }

        let ptr = GetWindowLongPtrW(h_wnd, GWLP_USERDATA.0);
        if ptr != 0 {
            let bridge = &*(ptr as *const EventBridge);
            let handler = &*(bridge.handler);
            let res = handler.wnd_proc(h_wnd, msg, w_param, l_param);
            if msg == WM_NCDESTROY {
                // make sure bridge is dropped
                let _ = Box::<EventBridge>::from_raw(ptr as *mut EventBridge);
            }
            return res;
        }

        DefWindowProcW(h_wnd, msg, w_param, l_param)
    }
}

pub fn enable_full_dpi_support(hwnd: HWND) {
    unsafe {
        let module = LoadLibraryW("User32.dll");
        if module.0 == 0 {
            return;
        }
        let enable = GetProcAddress(module, "EnableNonClientDpiScaling");
        if let Some(enable) = enable {
            let fnn: extern "system" fn(HWND) -> BOOL = std::mem::transmute(enable);
            fnn(hwnd);
        }

        FreeLibrary(module);
    }
}
