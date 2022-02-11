use std::{
    cell::{Cell, RefCell},
    mem,
    rc::{Rc, Weak},
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::{
        Dwm::DwmExtendFrameIntoClientArea,
        Gdi::{ClientToScreen, ScreenToClient},
    },
    UI::{
        Controls::MARGINS,
        Input::KeyboardAndMouse::ReleaseCapture,
        WindowsAndMessaging::{
            DestroyWindow, EnableMenuItem, GetSystemMenu, GetWindowLongW, GetWindowPlacement,
            GetWindowRect, IsWindowVisible, SendMessageW, SetForegroundWindow, SetWindowLongW,
            SetWindowPlacement, SetWindowPos, SetWindowTextW, ShowWindow, GWL_EXSTYLE, GWL_STYLE,
            HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT, HTTOP,
            HTTOPLEFT, HTTOPRIGHT, HTTRANSPARENT, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOP,
            HWND_TOPMOST, MF_BYCOMMAND, MF_DISABLED, MF_ENABLED, MF_GRAYED, SC_CLOSE,
            SHOW_WINDOW_CMD, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_NORMAL, SW_SHOW, WINDOWPLACEMENT,
            WINDOWPOS, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE,
            WM_DWMCOMPOSITIONCHANGED, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDOWN,
            WM_WINDOWPOSCHANGING, WS_BORDER, WS_CAPTION, WS_DLGFRAME, WS_EX_LAYOUTRTL,
            WS_EX_NOREDIRECTIONBITMAP, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPEDWINDOW,
            WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
        },
    },
};

use crate::{
    shell::{
        api_model::{
            WindowFrame, WindowGeometry, WindowGeometryFlags, WindowGeometryRequest, WindowStyle,
        },
        platform::error::PlatformError,
        IPoint, IRect, ISize, Point, Rect, Size,
    },
    util::OkLog,
};

use super::{
    display::Displays,
    error::PlatformResult,
    flutter_sys::{FlutterDesktopGetDpiForHWND, FlutterDesktopGetDpiForMonitor},
    util::{as_u8_slice, BoolResultExt, GET_X_LPARAM, GET_Y_LPARAM},
};

pub struct WindowBaseState {
    hwnd: HWND,
    min_frame_size: RefCell<Size>,
    max_frame_size: RefCell<Size>,
    min_content_size: RefCell<Size>,
    max_content_size: RefCell<Size>,
    delegate: Weak<dyn WindowDelegate>,
    style: RefCell<WindowStyle>,
    pending_show_cmd: Cell<SHOW_WINDOW_CMD>,
    last_window_pos: RefCell<Option<WINDOWPOS>>,
}

const LARGE_SIZE: f64 = 64.0 * 1024.0;

impl WindowBaseState {
    pub fn new(hwnd: HWND, delegate: Weak<dyn WindowDelegate>) -> Self {
        Self {
            hwnd,
            delegate,
            min_frame_size: RefCell::new(Size::wh(0.0, 0.0)),
            max_frame_size: RefCell::new(Size::wh(LARGE_SIZE, LARGE_SIZE)),
            min_content_size: RefCell::new(Size::wh(0.0, 0.0)),
            max_content_size: RefCell::new(Size::wh(LARGE_SIZE, LARGE_SIZE)),
            style: Default::default(),
            pending_show_cmd: Cell::new(SW_SHOW),
            last_window_pos: RefCell::new(None),
        }
    }

    pub fn hide(&self) -> PlatformResult<()> {
        unsafe { ShowWindow(self.hwnd, SW_HIDE).as_platform_result() }
    }

    pub fn activate(&self) -> PlatformResult<bool> {
        unsafe {
            SetWindowPos(
                self.hwnd,
                match self.style.borrow().always_on_top {
                    true => HWND_TOPMOST,
                    false => HWND_TOP,
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            )
        };
        unsafe { Ok(SetForegroundWindow(self.hwnd).into()) }
    }

    pub fn deactivate(&self) -> PlatformResult<bool> {
        unsafe {
            Ok(SetWindowPos(
                self.hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            )
            .into())
        }
    }

    pub fn show<F>(&self, callback: F) -> PlatformResult<()>
    where
        F: FnOnce() + 'static,
    {
        unsafe {
            ShowWindow(self.hwnd, self.pending_show_cmd.get()); // false is not an error
        }
        self.pending_show_cmd.set(SW_SHOW);
        callback();
        Ok(())
    }

    pub fn is_visible(&self) -> PlatformResult<bool> {
        unsafe { Ok(IsWindowVisible(self.hwnd).into()) }
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        let geometry = geometry.filtered_by_preference();

        let mut res = WindowGeometryFlags {
            ..Default::default()
        };

        if geometry.content_origin.is_some()
            || geometry.content_size.is_some()
            || geometry.frame_origin.is_some()
            || geometry.frame_size.is_some()
        {
            self.set_bounds_geometry(&geometry, &mut res)?;

            // There's no set_content_rect in winapi, so this is best effort implementation
            // that tries to deduce future content rect from current content rect and frame rect
            // in case it's wrong (i.e. display with different DPI or frame size change after reposition)
            // it will retry once again
            if res.content_origin || res.content_size {
                let content_rect = self.content_rect_for_frame_rect(&self.get_frame_rect()?)?;
                if (res.content_origin
                    && content_rect.origin() != *geometry.content_origin.as_ref().unwrap())
                    || (res.content_size
                        && content_rect.size() != *geometry.content_size.as_ref().unwrap())
                {
                    // retry
                    self.set_bounds_geometry(&geometry, &mut res)?;
                }
            }
        }

        if let Some(size) = geometry.min_frame_size {
            self.min_frame_size.replace(size);
            res.min_frame_size = true;
        }

        if let Some(size) = geometry.max_frame_size {
            self.max_frame_size.replace(size);
            res.max_frame_size = true;
        }

        if let Some(size) = geometry.min_content_size {
            self.min_content_size.replace(size);
            res.min_content_size = true;
        }

        if let Some(size) = geometry.max_content_size {
            self.max_content_size.replace(size);
            res.max_content_size = true;
        }

        Ok(res)
    }

    fn set_bounds_geometry(
        &self,
        geometry: &WindowGeometry,
        flags: &mut WindowGeometryFlags,
    ) -> PlatformResult<()> {
        let current_frame_rect = self.get_frame_rect()?;
        let current_content_rect = self.content_rect_for_frame_rect(&current_frame_rect)?;

        let content_offset = current_content_rect.to_local(&current_frame_rect.origin());
        let content_size_delta = current_frame_rect.size() - current_content_rect.size();

        let mut origin: Option<Point> = None;
        let mut size: Option<Size> = None;

        if let Some(frame_origin) = &geometry.frame_origin {
            origin.replace(frame_origin.clone());
            flags.frame_origin = true;
        }

        if let Some(frame_size) = &geometry.frame_size {
            size.replace(frame_size.clone());
            flags.frame_size = true;
        }

        if let Some(content_origin) = &geometry.content_origin {
            origin.replace(content_origin.translated(&content_offset));
            flags.content_origin = true;
        }

        if let Some(content_size) = &geometry.content_size {
            size.replace(content_size + &content_size_delta);
            flags.content_size = true;
        }

        let physical = IRect::origin_size(
            &self.to_physical(origin.as_ref().unwrap_or(&Point::xy(0.0, 0.0))),
            &size
                .as_ref()
                .unwrap_or(&Size::wh(0.0, 0.0))
                .scaled(self.get_scaling_factor())
                .into(),
        );

        let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
        if origin.is_none() {
            flags |= SWP_NOMOVE;
        }
        if size.is_none() {
            flags |= SWP_NOSIZE;
        }
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND(0),
                physical.x,
                physical.y,
                physical.width,
                physical.height,
                flags,
            )
            .as_platform_result()
        }
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        let frame_rect = self.get_frame_rect()?;
        let content_rect = self.content_rect_for_frame_rect(&frame_rect)?;

        Ok(WindowGeometry {
            frame_origin: Some(frame_rect.origin()),
            frame_size: Some(frame_rect.size()),
            content_origin: Some(content_rect.origin()),
            content_size: Some(content_rect.size()),
            min_frame_size: Some(self.min_frame_size.borrow().clone()),
            max_frame_size: Some(self.max_frame_size.borrow().clone()),
            min_content_size: Some(self.min_content_size.borrow().clone()),
            max_content_size: Some(self.max_content_size.borrow().clone()),
        })
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        Ok(WindowGeometryFlags {
            frame_origin: true,
            frame_size: true,
            content_origin: true,
            content_size: true,
            min_frame_size: true,
            max_frame_size: true,
            min_content_size: true,
            max_content_size: true,
        })
    }

    fn get_frame_rect(&self) -> PlatformResult<Rect> {
        let mut rect: RECT = Default::default();
        unsafe {
            GetWindowRect(self.hwnd, &mut rect as *mut _).as_platform_result()?;
        }
        let size: Size = ISize::wh(rect.right - rect.left, rect.bottom - rect.top).into();
        Ok(Rect::origin_size(
            &self.to_logical(&IPoint::xy(rect.left, rect.top)),
            &size.scaled(1.0 / self.get_scaling_factor()),
        ))
    }

    fn content_rect_for_frame_rect(&self, frame_rect: &Rect) -> PlatformResult<Rect> {
        let content_rect = IRect::origin_size(
            &self.to_physical(&frame_rect.top_left()),
            &frame_rect.size().scaled(self.get_scaling_factor()).into(),
        );
        let rect = RECT {
            left: content_rect.x,
            top: content_rect.y,
            right: content_rect.x2(),
            bottom: content_rect.y2(),
        };
        unsafe {
            SendMessageW(
                self.hwnd,
                WM_NCCALCSIZE as u32,
                WPARAM(0),
                LPARAM(&rect as *const _ as isize),
            );
        }
        let size: Size = ISize::wh(rect.right - rect.left, rect.bottom - rect.top).into();
        Ok(Rect::origin_size(
            &self.to_logical(&IPoint::xy(rect.left, rect.top)),
            &size.scaled(1.0 / self.get_scaling_factor()),
        ))
    }

    fn adjust_window_position(&self, position: &mut WINDOWPOS) -> PlatformResult<()> {
        let scale = self.get_scaling_factor();
        let frame_rect = self.get_frame_rect()?;
        let content_rect = self.content_rect_for_frame_rect(&frame_rect)?;

        let size_delta = frame_rect.size() - content_rect.size();

        let min_content = &*self.min_content_size.borrow() + &size_delta;
        let min_content: ISize = min_content.scaled(scale).into();

        let min_frame = self.min_frame_size.borrow();
        let min_frame: ISize = min_frame.scaled(scale).into();

        let min_size = ISize::wh(
            std::cmp::max(min_content.width, min_frame.width),
            std::cmp::max(min_content.height, min_frame.height),
        );

        let max_content = &*self.max_content_size.borrow() + &size_delta;
        let max_content: ISize = max_content.scaled(scale).into();

        let max_frame = self.max_frame_size.borrow();
        let max_frame: ISize = max_frame.scaled(scale).into();

        let max_size = ISize::wh(
            std::cmp::min(max_content.width, max_frame.width),
            std::cmp::min(max_content.height, max_frame.height),
        );

        position.cx = position.cx.clamp(min_size.width, max_size.width);
        position.cy = position.cy.clamp(min_size.height, max_size.height);

        Ok(())
    }

    pub fn close(&self) -> PlatformResult<()> {
        unsafe { DestroyWindow(self.hwnd).as_platform_result() }
    }

    pub fn local_to_global(&self, offset: &Point) -> IPoint {
        let scaled: IPoint = offset.scaled(self.get_scaling_factor()).into();
        self.local_to_global_physical(&scaled)
    }

    pub fn local_to_global_physical(&self, offset: &IPoint) -> IPoint {
        let mut point = POINT {
            x: offset.x,
            y: offset.y,
        };
        unsafe {
            ClientToScreen(self.hwnd, &mut point as *mut _);
        }
        IPoint::xy(point.x, point.y)
    }

    pub fn global_to_local(&self, offset: &IPoint) -> Point {
        let local: Point = self.global_to_local_physical(offset).into();
        local.scaled(1.0 / self.get_scaling_factor())
    }

    pub fn global_to_local_physical(&self, offset: &IPoint) -> IPoint {
        let mut point = POINT {
            x: offset.x,
            y: offset.y,
        };
        unsafe {
            ScreenToClient(self.hwnd, &mut point as *mut _);
        }
        IPoint::xy(point.x, point.y)
    }

    fn to_physical(&self, offset: &Point) -> IPoint {
        Displays::get_displays()
            .convert_logical_to_physical(offset)
            .unwrap_or_else(|| offset.clone().into())
    }

    fn to_logical(&self, offset: &IPoint) -> Point {
        Displays::get_displays()
            .convert_physical_to_logical(offset)
            .unwrap_or_else(|| offset.clone().into())
    }

    pub fn is_rtl(&self) -> bool {
        let style = WINDOW_EX_STYLE(unsafe { GetWindowLongW(self.hwnd, GWL_EXSTYLE) } as u32);
        style & WS_EX_LAYOUTRTL == WS_EX_LAYOUTRTL
    }

    pub fn get_scaling_factor(&self) -> f64 {
        unsafe { FlutterDesktopGetDpiForHWND(self.hwnd) as f64 / 96.0 }
    }

    #[allow(unused)]
    fn get_scaling_factor_for_monitor(&self, monitor: isize) -> f64 {
        unsafe { FlutterDesktopGetDpiForMonitor(monitor) as f64 / 96.0 }
    }

    fn delegate(&self) -> Rc<dyn WindowDelegate> {
        // delegate owns us so unwrap is safe here
        self.delegate.upgrade().unwrap()
    }

    unsafe fn set_close_enabled(&self, enabled: bool) {
        let menu = GetSystemMenu(self.hwnd, false);
        if enabled {
            EnableMenuItem(menu, SC_CLOSE, MF_BYCOMMAND | MF_ENABLED);
        } else {
            EnableMenuItem(
                menu,
                SC_CLOSE as u32,
                MF_BYCOMMAND | MF_DISABLED | MF_GRAYED,
            );
        }
    }

    pub fn update_dwm_frame(&self) -> PlatformResult<()> {
        let margin = match self.style.borrow().frame {
            WindowFrame::Regular => 0, // already has shadow
            WindowFrame::NoTitle => 1, // neede for window shadow
            WindowFrame::NoFrame => 0, // neede for transparency
        };

        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: margin,
            cyBottomHeight: 0,
        };
        unsafe {
            DwmExtendFrameIntoClientArea(self.hwnd, &margins as *const _).map_err(|e| e.into())
        }
    }

    pub fn set_title(&self, title: String) -> PlatformResult<()> {
        unsafe {
            SetWindowTextW(self.hwnd, title);
        }
        Ok(())
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        *self.style.borrow_mut() = style.clone();
        unsafe {
            let mut s = WINDOW_STYLE(GetWindowLongW(self.hwnd, GWL_STYLE) as u32);
            s &= WINDOW_STYLE(!(WS_OVERLAPPEDWINDOW | WS_DLGFRAME).0);

            if style.frame == WindowFrame::Regular {
                s |= WS_CAPTION;
                if style.can_resize {
                    s |= WS_THICKFRAME;
                }
            }

            if style.frame == WindowFrame::NoTitle {
                s |= WS_CAPTION;
                if style.can_resize {
                    s |= WS_THICKFRAME;
                } else {
                    s |= WS_BORDER;
                }
            }

            if style.frame == WindowFrame::NoFrame {
                s |= WS_POPUP
            }

            s |= WS_SYSMENU;
            self.set_close_enabled(style.can_close);
            if style.can_maximize && style.can_resize {
                s |= WS_MAXIMIZEBOX;
            }
            if style.can_minimize {
                s |= WS_MINIMIZEBOX;
            }

            SetWindowLongW(self.hwnd, GWL_STYLE, s.0 as i32);
            SetWindowPos(
                self.hwnd,
                HWND(0),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
            )
            .as_platform_result()?;

            self.update_dwm_frame()?;
        }
        unsafe {
            SetWindowPos(
                self.hwnd,
                match style.always_on_top {
                    true => HWND_TOPMOST,
                    false => HWND_NOTOPMOST,
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            )
        };
        Ok(())
    }

    pub fn minimize(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_MINIMIZE);
        }
    }

    pub fn maximize(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_MAXIMIZE);
        }
    }

    pub fn restore(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_NORMAL);
        }
    }

    pub fn save_position_to_string(&self) -> PlatformResult<String> {
        unsafe {
            let mut placement = WINDOWPLACEMENT {
                length: mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            if GetWindowPlacement(self.hwnd, &mut placement as *mut _).as_bool() {
                let buffer = as_u8_slice(&placement);
                Ok(base64::encode(buffer))
            } else {
                Ok(String::new())
            }
        }
    }

    pub fn restore_position_from_string(&self, position: String) -> PlatformResult<()> {
        let buffer = base64::decode(&position).map_err(|e| PlatformError::OtherError {
            error: format!("{}", e),
        })?;
        if buffer.len() != mem::size_of::<WINDOWPLACEMENT>() {
            return Err(PlatformError::OtherError {
                error: "Invalid placement string".into(),
            });
        }
        let placement = buffer.as_ptr() as *mut WINDOWPLACEMENT;
        unsafe {
            let placement = &mut *placement;
            if placement.length != mem::size_of::<WINDOWPLACEMENT>() as u32 {
                return Err(PlatformError::OtherError {
                    error: "Invalid placement string".into(),
                });
            }
            if !self.is_visible()? {
                self.pending_show_cmd.set(placement.showCmd);

                placement.showCmd = SW_HIDE;
            }
            SetWindowPlacement(self.hwnd, placement as *const _);
        }

        Ok(())
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        unsafe {
            println!("Perform window drag!");
            ReleaseCapture();
            SendMessageW(
                self.hwnd,
                WM_NCLBUTTONDOWN as u32,
                WPARAM(HTCAPTION as usize),
                LPARAM(0),
            );
        }
        Ok(())
    }

    pub fn has_redirection_surface(&self) -> bool {
        let style = WINDOW_EX_STYLE(unsafe { GetWindowLongW(self.hwnd, GWL_EXSTYLE) } as u32);
        (style & WS_EX_NOREDIRECTIONBITMAP).0 == 0
    }

    pub fn remove_border(&self) -> bool {
        self.style.borrow().frame == WindowFrame::NoTitle
    }

    fn do_hit_test(&self, x: i32, y: i32) -> u32 {
        let mut win_rect = RECT::default();
        unsafe {
            GetWindowRect(self.hwnd, &mut win_rect as *mut _);
        }

        let border_width = (7.0 * self.get_scaling_factor()) as i32;

        if x < win_rect.left + border_width && y < win_rect.top + border_width {
            HTTOPLEFT
        } else if x > win_rect.right - border_width && y < win_rect.top + border_width {
            HTTOPRIGHT
        } else if y < win_rect.top + border_width {
            HTTOP
        } else if x < win_rect.left + border_width && y > win_rect.bottom - border_width {
            HTBOTTOMLEFT
        } else if x > win_rect.right - border_width && y > win_rect.bottom - border_width {
            HTBOTTOMRIGHT
        } else if y > win_rect.bottom - border_width {
            HTBOTTOM
        } else if x < win_rect.left + border_width {
            HTLEFT
        } else if x > win_rect.right - border_width {
            HTRIGHT
        } else {
            HTCLIENT
        }
    }

    pub fn handle_message(
        &self,
        _h_wnd: HWND,
        msg: u32,
        _w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_CLOSE => {
                self.delegate().should_close();
                Some(LRESULT(0))
            }
            WM_DESTROY => {
                self.delegate().will_close();
                None
            }
            WM_DISPLAYCHANGE => {
                Displays::displays_changed();
                None
            }
            WM_WINDOWPOSCHANGING => {
                let position = unsafe { &mut *(l_param.0 as *mut WINDOWPOS) };
                let pos_before = *position;
                self.adjust_window_position(position).ok_log();

                if let Some(ref prev_window_pos) = *self.last_window_pos.borrow() {
                    if pos_before.cx < position.cx {
                        // fix window drift when resizing left border past minimum size
                        if position.x != prev_window_pos.x {
                            position.x = prev_window_pos.x + prev_window_pos.cx - position.cx;
                        }
                    }
                    if pos_before.cy < position.cy {
                        // fix window drift when resizing top border past minimum size
                        if position.y != prev_window_pos.y {
                            position.y = prev_window_pos.y + prev_window_pos.cy - position.cy;
                        }
                    }
                }
                self.last_window_pos.borrow_mut().replace(*position);
                None
            }
            WM_DWMCOMPOSITIONCHANGED => {
                self.update_dwm_frame().ok_log();
                None
            }
            WM_NCCALCSIZE => {
                if self.remove_border() {
                    Some(LRESULT(1))
                } else {
                    None
                }
            }
            WM_NCHITTEST => {
                if self.remove_border() {
                    let res = self.do_hit_test(GET_X_LPARAM(l_param), GET_Y_LPARAM(l_param));
                    Some(LRESULT(res as isize))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn handle_child_message(
        &self,
        _h_wnd: HWND,
        msg: u32,
        _w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_NCHITTEST => {
                if self.remove_border() {
                    let res = self.do_hit_test(GET_X_LPARAM(l_param), GET_Y_LPARAM(l_param));
                    if res != HTCLIENT {
                        Some(LRESULT(HTTRANSPARENT as isize))
                    } else {
                        Some(LRESULT(res as isize))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub trait WindowDelegate {
    fn should_close(&self);
    fn will_close(&self);
}
