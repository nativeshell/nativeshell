use std::{cell::RefCell, rc::Weak};

use gdk::{Display, Monitor, Rectangle};
use gdk_sys::GdkMonitor;
use glib::translate::ToGlibPtr;

use crate::shell::{api_model::Screen, screen_manager::ScreenManagerDelegate, Point, Rect};

use super::error::PlatformResult;

pub struct PlatformScreenManager {}

impl PlatformScreenManager {
    pub fn new(delegate: Weak<RefCell<dyn ScreenManagerDelegate>>) -> Self {
        if let Some(display) = Display::default() {
            let d = delegate.clone();
            display.default_screen().connect_size_changed(move |_| {
                if let Some(d) = d.upgrade() {
                    d.borrow().screen_configuration_changed();
                }
            });
            // TODO(knopp): Are these necessary?
            let d = delegate.clone();
            display.connect_monitor_added(move |_, _| {
                if let Some(d) = d.upgrade() {
                    d.borrow().screen_configuration_changed();
                }
            });
            let d = delegate;
            display.connect_monitor_removed(move |_, _| {
                if let Some(d) = d.upgrade() {
                    d.borrow().screen_configuration_changed();
                }
            });
        }
        Self {}
    }

    pub fn get_monitor_id(monitor: &Monitor) -> i64 {
        let res: *mut GdkMonitor = monitor.to_glib_none().0;
        res as i64
    }

    pub fn get_screens(&self) -> PlatformResult<Vec<Screen>> {
        let mut res = Vec::new();
        if let Some(display) = Display::default() {
            for i in 0..display.n_monitors() {
                let monitor = display.monitor(i);
                if let Some(monitor) = monitor {
                    res.push(Self::screen_for_monitor(&monitor))
                }
            }
        }
        Ok(res)
    }

    fn screen_for_monitor(monitor: &Monitor) -> Screen {
        Screen {
            id: Self::get_monitor_id(monitor),
            frame: Self::convert_rect(monitor.geometry()),
            work_area: Self::convert_rect(monitor.workarea()),
            scaling_factor: monitor.scale_factor() as f64,
        }
    }

    fn convert_rect(rect: Rectangle) -> Rect {
        Rect::xywh(
            rect.x as f64,
            rect.y as f64,
            rect.width as f64,
            rect.height as f64,
        )
    }

    pub fn get_main_screen(&self) -> PlatformResult<i64> {
        if let Some(display) = Display::default() {
            let monitor = display.primary_monitor();
            Ok(monitor.as_ref().map(Self::get_monitor_id).unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    pub fn logical_to_system(&self, offset: Point) -> PlatformResult<Point> {
        Ok(offset)
    }

    pub fn system_to_logical(&self, offset: Point) -> PlatformResult<Point> {
        Ok(offset)
    }
}
