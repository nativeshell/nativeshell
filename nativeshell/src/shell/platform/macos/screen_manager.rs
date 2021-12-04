use std::{cell::RefCell, rc::Weak};

use block::ConcreteBlock;
use cocoa::{
    appkit::NSScreen,
    base::{id, nil},
    foundation::{NSArray, NSDictionary},
};
use objc::{class, msg_send, rc::autoreleasepool, sel, sel_impl};

use crate::shell::{api_model::Screen, screen_manager::ScreenManagerDelegate};

use super::utils::to_nsstring;

pub struct PlatformScreenManager {}

impl PlatformScreenManager {
    pub fn new(delegate: Weak<RefCell<dyn ScreenManagerDelegate>>) -> Self {
        autoreleasepool(|| unsafe {
            let center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
            let queue: id = msg_send![class!(NSOperationQueue), mainQueue];
            let block = ConcreteBlock::new(move || {
                if let Some(delegate) = delegate.upgrade() {
                    delegate.borrow().screen_configuration_changed();
                }
            });
            let block = block.copy();
            let name = to_nsstring("NSApplicationDidChangeScreenParametersNotification");
            let () = msg_send![center,
                        addObserverForName:*name object:nil queue: queue usingBlock:&*block];
        });
        Self {}
    }

    pub(super) unsafe fn get_screen_id(screen: id) -> i64 {
        let description = NSScreen::deviceDescription(screen);
        let device_id = NSDictionary::objectForKey_(description, *to_nsstring("NSScreenNumber"));
        msg_send![device_id, longLongValue]
    }

    pub fn get_screens(&self) -> Vec<Screen> {
        let mut res = Vec::new();
        autoreleasepool(|| unsafe {
            let screens = NSScreen::screens(nil);
            for i in 0..NSArray::count(screens) {
                let screen = NSArray::objectAtIndex(screens, i);
                let s = Screen {
                    id: Self::get_screen_id(screen),
                    main: NSScreen::mainScreen(nil) == screen,
                    frame: NSScreen::frame(screen).into(),
                    visible_frame: NSScreen::visibleFrame(screen).into(),
                    scaling_factor: NSScreen::backingScaleFactor(screen),
                };
                res.push(s);
            }
        });
        res
    }
}
