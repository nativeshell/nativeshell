use std::{ptr, rc::Weak};

use gdk::Event;
use glib::ObjectExt;

use crate::shell::platform::window::PlatformWindow;

use super::error::{PlatformError, PlatformResult};

pub fn init_platform() -> PlatformResult<()> {
    gtk::init().map_err(|e| PlatformError::GLibError {
        message: e.message.into(),
    })?;

    Event::set_handler(Some(|e: &mut Event| {
        let win = e.window().map(|w| w.toplevel());

        if let Some(win) = win {
            let platform_window: Option<ptr::NonNull<Weak<PlatformWindow>>> =
                unsafe { win.data("nativeshell_platform_window") };

            if let Some(platform_window) =
                platform_window.and_then(|w| unsafe { w.as_ref() }.upgrade())
            {
                platform_window.on_event(e);
            }
        }

        gtk::main_do_event(e);
    }));

    Ok(())
}
