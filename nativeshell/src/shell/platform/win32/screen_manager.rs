use std::{cell::RefCell, rc::Weak};

use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST},
    UI::WindowsAndMessaging::GetForegroundWindow,
};

use crate::shell::{api_model::Screen, screen_manager::ScreenManagerDelegate, IPoint, Point};

use super::{
    display::Displays,
    error::{PlatformError, PlatformResult},
};

pub struct PlatformScreenManager {}

impl PlatformScreenManager {
    pub fn new(delegate: Weak<RefCell<dyn ScreenManagerDelegate>>) -> Self {
        Displays::on_displays_changed(move || {
            if let Some(delegate) = delegate.upgrade() {
                delegate.borrow().screen_configuration_changed();
            }
        });
        Self {}
    }

    pub fn get_screens(&self) -> PlatformResult<Vec<Screen>> {
        let displays = Displays::get_displays();
        Ok(displays
            .displays
            .iter()
            .map(|d| Screen {
                id: d.id,
                frame: d.logical.clone(),
                work_area: d.work.clone(),
                scaling_factor: d.scale,
            })
            .collect())
    }

    pub fn get_main_screen(&self) -> PlatformResult<i64> {
        let window = unsafe { GetForegroundWindow() };
        if window.0 == 0 {
            return Ok(0);
        }
        Self::screen_id_from_hwnd(window)
    }

    pub fn screen_id_from_hwnd(window: HWND) -> PlatformResult<i64> {
        let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
        let displays = Displays::get_displays();
        let res = displays
            .displays
            .iter()
            .find(|d| d.handle == monitor)
            .map(|d| d.id);
        Ok(res.unwrap_or(0))
    }

    pub fn logical_to_system(&self, offset: Point) -> PlatformResult<Point> {
        let displays = Displays::get_displays();
        let res = displays.convert_logical_to_physical(&offset);
        res.map_or_else(
            || Err(PlatformError::OffsetOutOfScreenBounds),
            |f| Ok(f.into()),
        )
    }

    pub fn system_to_logical(&self, offset: Point) -> PlatformResult<Point> {
        let displays = Displays::get_displays();
        let res = displays.convert_physical_to_logical(&IPoint::from(offset));
        res.map_or_else(|| Err(PlatformError::OffsetOutOfScreenBounds), Ok)
    }
}
