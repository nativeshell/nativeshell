use std::{cell::RefCell, rc::Weak};

use block::ConcreteBlock;
use cocoa::{
    appkit::NSScreen,
    base::{id, nil},
    foundation::{NSArray, NSDictionary},
};
use objc::{class, msg_send, rc::autoreleasepool, sel, sel_impl};

use crate::shell::{api_model::Screen, screen_manager::ScreenManagerDelegate, Point, Rect};

use super::{
    error::PlatformResult,
    utils::{global_screen_frame, to_nsstring},
};

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

    fn flip_rect(rect: &Rect, global_screen_frame: &Rect) -> Rect {
        Rect {
            x: rect.x,
            y: global_screen_frame.y2() - rect.y2(),
            width: rect.width,
            height: rect.height,
        }
    }

    pub fn get_screens(&self) -> PlatformResult<Vec<Screen>> {
        let mut res = Vec::new();
        autoreleasepool(|| unsafe {
            let global_frame = global_screen_frame();
            let screens = NSScreen::screens(nil);
            for i in 0..NSArray::count(screens) {
                let screen = NSArray::objectAtIndex(screens, i);
                let s = Screen {
                    id: Self::get_screen_id(screen),
                    frame: Self::flip_rect(&NSScreen::frame(screen).into(), &global_frame),
                    work_area: Self::flip_rect(
                        &NSScreen::visibleFrame(screen).into(),
                        &global_frame,
                    ),
                    scaling_factor: NSScreen::backingScaleFactor(screen),
                };
                res.push(s);
            }
        });
        Ok(res)
    }

    pub fn get_main_screen(&self) -> PlatformResult<i64> {
        autoreleasepool(|| unsafe {
            let screen = NSScreen::mainScreen(nil);
            Ok(Self::get_screen_id(screen))
        })
    }

    // macOS does the mapping
    pub fn logical_to_system(&self, offset: Point) -> PlatformResult<Point> {
        Ok(offset)
    }

    // macOS does the mapping
    pub fn system_to_logical(&self, offset: Point) -> PlatformResult<Point> {
        Ok(offset)
    }
}
