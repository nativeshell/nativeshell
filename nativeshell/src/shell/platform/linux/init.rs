use std::rc::{Rc, Weak};

use gdk::{Event, WindowExt};
use glib::ObjectExt;

use crate::shell::{platform::window::PlatformWindow, Context};

use super::error::{PlatformError, PlatformResult};

pub fn init_platform(_context: Rc<Context>) -> PlatformResult<()> {
    gtk::init().map_err(|e| PlatformError::GLibError {
        message: e.message.into(),
    })?;

    Event::set_handler(Some(|e: &mut Event| {
        let win = e.get_window().map(|w| w.get_toplevel());

        if let Some(win) = win {
            let platform_window: Option<&Weak<PlatformWindow>> =
                unsafe { win.get_data("nativeshell_platform_window") };
            if let Some(platform_window) = platform_window.and_then(|w| w.upgrade()) {
                platform_window.on_event(e);
            }
        }

        gtk::main_do_event(e);
    }));

    Ok(())
}
