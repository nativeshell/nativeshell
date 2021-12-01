use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use block::ConcreteBlock;
use cocoa::{
    appkit::{
        NSEvent, NSEventMask, NSEventModifierFlags,
        NSEventType::{NSLeftMouseDown, NSLeftMouseUp, NSRightMouseDown, NSRightMouseUp},
        NSScreen, NSStatusBar, NSVariableStatusItemLength, NSView, NSWindow,
    },
    base::{id, nil, NO, YES},
};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use crate::{
    shell::{
        api_model::{ImageData, StatusItemActionType, StatusItemGeometry},
        status_item_manager::{StatusItemDelegate, StatusItemHandle},
        EngineHandle, Point, Size,
    },
    util::LateRefCell,
    Context,
};

use super::{menu::PlatformMenu, utils::ns_image_from};

pub struct PlatformStatusItem {
    handle: StatusItemHandle,
    delegate: Weak<RefCell<dyn StatusItemDelegate>>,
    pub(crate) engine: EngineHandle,
    status_item: StrongPtr,
    weak_self: LateRefCell<Weak<PlatformStatusItem>>,
}

impl PlatformStatusItem {
    pub fn new(
        handle: StatusItemHandle,
        delegate: Weak<RefCell<dyn StatusItemDelegate>>,
        engine: EngineHandle,
    ) -> Self {
        let item = autoreleasepool(|| unsafe {
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let item = NSStatusBar::statusItemWithLength_(status_bar, NSVariableStatusItemLength);
            StrongPtr::retain(item)
        });
        Self {
            handle,
            delegate,
            engine,
            status_item: item,
            weak_self: LateRefCell::new(),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformStatusItem>) {
        self.weak_self.set(weak);
    }

    fn window(&self) -> id {
        unsafe {
            let button: id = msg_send![*self.status_item, button];
            msg_send![button, window]
        }
    }

    fn on_event(&self, event: id) -> id {
        let delegate = self.delegate.upgrade();
        if let Some(delegate) = delegate {
            let event_type = unsafe { NSEvent::eventType(event) };
            let action = match event_type {
                NSLeftMouseDown => Some(StatusItemActionType::LeftMouseDown),
                NSLeftMouseUp => Some(StatusItemActionType::LeftMouseUp),
                NSRightMouseDown => Some(StatusItemActionType::RightMouseDown),
                NSRightMouseUp => Some(StatusItemActionType::RightMouseUp),
                _ => None,
            };
            if let Some(action) = action {
                delegate.borrow().on_action(self.handle, action);
                return nil;
            }
        }
        event
    }

    pub fn set_image(&self, image: Vec<ImageData>) {
        autoreleasepool(move || unsafe {
            let image = ns_image_from(image);
            let () = msg_send![*image, setTemplate: YES];
            let button: id = msg_send![*self.status_item, button];
            let () = msg_send![button, setImage: *image];
        });
    }

    pub fn show_menu<F>(&self, menu: Rc<PlatformMenu>, on_done: F)
    where
        F: FnOnce() + 'static,
    {
        autoreleasepool(move || unsafe {
            let status_item = self.status_item.clone();
            let button: id = msg_send![*self.status_item, button];
            let context = Context::current().unwrap();

            context
                .run_loop
                .borrow()
                .schedule_now(move || {
                    let () = msg_send![*status_item, setMenu:*menu.menu];
                    let () = msg_send![button, performClick: nil];
                    let () = msg_send![*status_item, setMenu: nil];
                    on_done();
                })
                .detach();
        });
    }

    pub fn set_highlighted(&self, highlighted: bool) {
        autoreleasepool(move || unsafe {
            let button: id = msg_send![*self.status_item, button];
            let value = if highlighted { YES } else { NO };
            let () = msg_send![button, highlight: value];
        });
    }

    pub fn get_geometry(&self) -> StatusItemGeometry {
        autoreleasepool(move || unsafe {
            let button: id = msg_send![*self.status_item, button];
            let window: id = msg_send![button, window];
            let window_frame = NSWindow::frame(window);
            let button_frame = NSView::frame(button);
            let screen_frame = NSScreen::frame(NSWindow::screen(window));

            // println!(
            //     "button_frame: {:?} {:?}",
            //     button_frame.origin.y, button_frame.size.height
            // );
            // println!(
            //     "screen_frame: {:?} {:?}",
            //     screen_frame.origin.y, screen_frame.size.height
            // );
            // println!(
            //     "window_frame: {:?} {:?}",
            //     window_frame.origin.y, window_frame.size.height
            // );

            StatusItemGeometry {
                origin: Point::xy(
                    window_frame.origin.x + button_frame.origin.x,
                    screen_frame.size.height
                        - (window_frame.origin.y + button_frame.origin.y)
                        - button_frame.size.height,
                ),
                size: Size::wh(button_frame.size.width, button_frame.size.height),
            }
        })
    }
}

pub struct PlatformStatusItemManager {
    event_monitor: LateRefCell<StrongPtr>,
    items: RefCell<Vec<Rc<PlatformStatusItem>>>,
}

impl PlatformStatusItemManager {
    pub fn new() -> Self {
        Self {
            event_monitor: LateRefCell::new(),
            items: RefCell::new(Vec::new()),
        }
    }

    fn on_event(&self, event: id) -> id {
        let modifier_flags = unsafe { NSEvent::modifierFlags(event) };
        // allow command draggging
        if modifier_flags.contains(NSEventModifierFlags::NSCommandKeyMask) {
            return event;
        }

        let window: id = unsafe { msg_send![event, window] };
        for item in self.items.borrow().iter() {
            if item.window() == window {
                return item.on_event(event);
            }
        }
        event
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformStatusItemManager>) {
        let block = move |event: id| {
            if let Some(s) = weak.upgrade() {
                s.on_event(event)
            } else {
                event
            }
        };
        let block = ConcreteBlock::new(block);
        let block = block.copy();

        let mask = NSEventMask::NSLeftMouseDownMask
            | NSEventMask::NSLeftMouseUpMask
            | NSEventMask::NSRightMouseDownMask
            | NSEventMask::NSRightMouseUpMask;
        unsafe {
            let monitor: id = msg_send![class!(NSEvent), addLocalMonitorForEventsMatchingMask:mask handler:&*block];
            self.event_monitor.set(StrongPtr::new(monitor));
        }
    }

    pub fn register_status_item(&self, item: &Rc<PlatformStatusItem>) {
        self.items.borrow_mut().push(item.clone());
    }

    pub fn unregister_status_item(&self, item: &Rc<PlatformStatusItem>) {
        self.items.borrow_mut().retain(|i| !Rc::ptr_eq(i, &item));
    }
}

impl Drop for PlatformStatusItemManager {
    fn drop(&mut self) {
        unsafe {
            let () = msg_send![class!(NSEvent), removeMonitor:*self.event_monitor.borrow().clone()];
        }
    }
}
