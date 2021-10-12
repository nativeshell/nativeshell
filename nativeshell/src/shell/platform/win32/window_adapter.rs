use super::{all_bindings::*, util::direct_composition_supported};
use lazy_static::lazy_static;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

struct Global {
    window_class: RefCell<Weak<WindowClass>>,
}

// Send is required when other dependencies apply the lazy_static feature 'spin_no_std'
unsafe impl Send for Global {}
unsafe impl Sync for Global {}

lazy_static! {
    static ref GLOBAL: Global = Global {
        window_class: RefCell::new(Weak::new()),
    };
}

struct WindowClass {
    pub class_name: String,
}

impl WindowClass {
    pub fn get() -> Rc<Self> {
        let res = GLOBAL.window_class.borrow().upgrade();
        match res {
            Some(class) => class,
            None => {
                let res = Rc::new(Self::new());
                GLOBAL.window_class.replace(Rc::downgrade(&res));
                res
            }
        }
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
            if msg == WM_NCDESTROY as u32 {
                // make sure bridge is dropped
                Box::<EventBridge>::from_raw(ptr as *mut EventBridge);
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
