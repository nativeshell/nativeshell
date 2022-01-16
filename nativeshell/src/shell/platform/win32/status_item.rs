use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::DeleteObject,
    UI::{
        Shell::{
            Shell_NotifyIconGetRect, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIM_ADD, NIM_DELETE,
            NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICONDATAW_0, NOTIFYICONIDENTIFIER,
            NOTIFYICON_VERSION_4,
        },
        WindowsAndMessaging::{
            CreateIconIndirect, DestroyIcon, HICON, ICONINFO, WM_LBUTTONDOWN, WM_LBUTTONUP,
            WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
        },
    },
};

use crate::{
    shell::{
        api_model::{ImageData, StatusItemActionType},
        platform::util::{HIWORD, LOWORD},
        status_item_manager::{StatusItemDelegate, StatusItemHandle},
        EngineHandle, IPoint, Point, Rect,
    },
    Context,
};

use super::{
    display::Displays,
    error::{PlatformError, PlatformResult},
    menu::PlatformMenu,
    run_loop::{PlatformRunLoopStatusItemDelegate, WM_STATUS_ITEM},
    util::image_data_to_hbitmap,
};

pub struct PlatformStatusItem {
    handle: StatusItemHandle,
    delegate: Weak<RefCell<dyn StatusItemDelegate>>,
    pub engine: EngineHandle,
    context: Context,
    image: RefCell<Vec<ImageData>>,
}

impl PlatformStatusItem {
    pub fn new(
        handle: StatusItemHandle,
        delegate: Weak<RefCell<dyn StatusItemDelegate>>,
        engine: EngineHandle,
        context: Context,
    ) -> Self {
        Self {
            handle,
            delegate,
            engine,
            context,
            image: RefCell::new(Vec::new()),
        }
    }

    fn hwnd(&self) -> HWND {
        self.context
            .get()
            .map(|c| c.run_loop.borrow().platform_run_loop.hwnd())
            .unwrap_or(HWND(0))
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformStatusItem>) {
        let init_data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            uID: self.handle.0 as u32,
            hWnd: self.hwnd(),
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            ..Default::default()
        };

        unsafe {
            Shell_NotifyIconW(NIM_ADD, &init_data as *const _);
            Shell_NotifyIconW(NIM_SETVERSION, &init_data as *const _);
        }
    }

    fn update(&self) {
        // choose the icon closest to max display scale
        let max_scale = Displays::get_displays()
            .displays
            .iter()
            .map(|d| d.scale)
            .reduce(f64::max)
            .unwrap_or(1.0);
        let ideal_height = (max_scale * 16.0).round() as i32;
        let icon = self
            .image
            .borrow()
            .iter()
            .min_by(|a, b| {
                let d1 = (a.height - ideal_height).abs();
                let d2 = (b.height - ideal_height).abs();
                d1.cmp(&d2)
            })
            .map(Self::image_to_icon)
            .unwrap_or(HICON(0));
        let mut flags = NIF_MESSAGE;
        if icon.0 != 0 {
            flags |= NIF_ICON;
        }
        let data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd(),
            uID: self.handle.0 as u32,
            uFlags: flags,
            uCallbackMessage: WM_STATUS_ITEM,
            hIcon: icon,
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            ..Default::default()
        };
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &data as *const _);

            if icon.0 != 0 {
                DestroyIcon(icon);
            }
        }
    }

    fn image_to_icon(image: &ImageData) -> HICON {
        let bitmap = image_data_to_hbitmap(image);
        let icon_info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: bitmap,
            hbmColor: bitmap,
        };
        let res = unsafe { CreateIconIndirect(&icon_info as *const _) };
        unsafe {
            DeleteObject(bitmap);
        };
        res
    }

    pub fn set_image(&self, image: Vec<ImageData>) -> PlatformResult<()> {
        self.image.replace(image);
        self.update();
        Ok(())
    }

    pub fn show_menu<F>(&self, menu: Rc<PlatformMenu>, on_done: F) {}

    pub fn set_highlighted(&self, _highlighted: bool) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn get_geometry(&self) -> PlatformResult<Rect> {
        let rect = self.get_rect()?;
        let displays = Displays::get_displays();
        let origin = IPoint::xy(rect.left, rect.top);
        let display = displays.display_for_physical_point(&origin);

        if let Some(display) = display {
            let origin = displays.convert_physical_to_logical(&origin);
            if let Some(origin) = origin {
                return Ok(Rect::xywh(
                    origin.x,
                    origin.y,
                    (rect.right - rect.left) as f64 / display.scale,
                    (rect.bottom - rect.top) as f64 / display.scale,
                ));
            }
        }
        Err(PlatformError::OffsetOutOfScreenBounds)
    }

    pub fn get_screen_id(&self) -> PlatformResult<i64> {
        let rect = self.get_rect()?;
        let displays = Displays::get_displays();
        let display = displays.display_for_physical_point(&IPoint::xy(rect.left, rect.top));
        Ok(display.map(|d| d.id).unwrap_or(0))
    }

    // returns rectangle in system coordinates
    fn get_rect(&self) -> PlatformResult<RECT> {
        unsafe {
            let id = NOTIFYICONIDENTIFIER {
                cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
                hWnd: self.hwnd(),
                uID: self.handle.0 as u32,
                guidItem: Default::default(),
            };
            Shell_NotifyIconGetRect(&id as *const _)
        }
        .map_err(PlatformError::from)
    }

    pub fn on_message(&self, msg: u32, x: u16, y: u16) {
        if msg == WM_MOUSEMOVE {
            return;
        }
        if let Ok(rect) = self.get_rect() {
            let displays = Displays::get_displays();
            let screen = displays.display_for_physical_point(&IPoint::xy(rect.left, rect.top));
            if let Some(screen) = screen {
                let x = (x as i32 - rect.left) as f64 / screen.scale;
                let y = (y as i32 - rect.top) as f64 / screen.scale;
                if let Some(action) = match msg {
                    WM_LBUTTONDOWN => Some(StatusItemActionType::LeftMouseDown),
                    WM_LBUTTONUP => Some(StatusItemActionType::LeftMouseUp),
                    WM_RBUTTONDOWN => Some(StatusItemActionType::RightMouseDown),
                    WM_RBUTTONUP => Some(StatusItemActionType::RightMouseUp),
                    _ => None,
                } {
                    if let Some(delegate) = self.delegate.upgrade() {
                        delegate
                            .borrow()
                            .on_action(self.handle, action, Point::xy(x, y));
                    }
                }
                // println!("X: {}, Y: {}", x, y);
            }
        }
        println!("w {:?} , {} {}", msg, x, y);
    }
}

impl Drop for PlatformStatusItem {
    fn drop(&mut self) {
        let delete_data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            uID: self.handle.0 as u32,
            hWnd: self.hwnd(),
            ..Default::default()
        };

        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &delete_data as *const _);
        }
    }
}

pub struct PlatformStatusItemManager {
    context: Context,
    items: RefCell<Vec<Rc<PlatformStatusItem>>>,
}

impl PlatformStatusItemManager {
    pub fn new(context: Context) -> Self {
        Self {
            context,
            items: RefCell::new(Vec::new()),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformStatusItemManager>) {
        if let Some(context) = self.context.get() {
            context
                .run_loop
                .borrow()
                .platform_run_loop
                .set_status_item_delegate(weak);
        }
    }

    pub fn crete_status_item(
        &self,
        handle: StatusItemHandle,
        delegate: Weak<RefCell<dyn StatusItemDelegate>>,
        engine: EngineHandle,
    ) -> Rc<PlatformStatusItem> {
        let res = Rc::new(PlatformStatusItem::new(
            handle,
            delegate,
            engine,
            self.context.clone(),
        ));
        self.items.borrow_mut().push(res.clone());
        res
    }

    pub fn unregister_status_item(&self, item: &Rc<PlatformStatusItem>) {
        self.items.borrow_mut().retain(|i| !Rc::ptr_eq(i, item));
    }
}

impl PlatformRunLoopStatusItemDelegate for PlatformStatusItemManager {
    fn on_status_item_message(
        &self,
        w_param: windows::Win32::Foundation::WPARAM,
        l_param: windows::Win32::Foundation::LPARAM,
    ) {
        let msg = LOWORD(l_param.0 as u32);
        let id = HIWORD(l_param.0 as u32);
        let x = LOWORD(w_param.0 as u32);
        let y = HIWORD(w_param.0 as u32);

        let item = self
            .items
            .borrow()
            .iter()
            .find(|i| i.handle.0 == id as i64)
            .cloned();
        if let Some(item) = item {
            item.on_message(msg as u32, x, y);
        }
    }
}
