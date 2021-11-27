use std::rc::{Rc, Weak};

use cocoa::{
    appkit::{NSStatusBar, NSVariableStatusItemLength},
    base::{id, nil, YES},
};
use objc::{
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use crate::{
    shell::{api_model::ImageData, status_item_manager::StatusItemHandle, Context, EngineHandle},
    util::LateRefCell,
};

use super::{menu::PlatformMenu, utils::ns_image_from};

pub struct PlatformStatusItem {
    context: Context,
    handle: StatusItemHandle,
    engine: EngineHandle,
    status_item: StrongPtr,
    weak_self: LateRefCell<Weak<PlatformStatusItem>>,
}

impl PlatformStatusItem {
    pub fn new(context: Context, handle: StatusItemHandle, engine: EngineHandle) -> Self {
        let item = autoreleasepool(|| unsafe {
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let item = NSStatusBar::statusItemWithLength_(status_bar, NSVariableStatusItemLength);
            StrongPtr::retain(item)
        });
        Self {
            context,
            handle,
            engine,
            status_item: item,
            weak_self: LateRefCell::new(),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformStatusItem>) {
        self.weak_self.set(weak);
    }

    pub fn set_image(&self, image: Vec<ImageData>) {
        autoreleasepool(move || unsafe {
            let image = ns_image_from(image);
            let () = msg_send![*image, setTemplate: YES];
            let button: id = msg_send![*self.status_item, button];
            let () = msg_send![button, setImage: *image];
        });
    }

    pub fn set_menu(&self, menu: Option<Rc<PlatformMenu>>) {
        autoreleasepool(move || unsafe {
            let () = msg_send![*self.status_item, setMenu:match menu {
                Some(menu) => *menu.menu,
                None => nil,
            }];
        });
    }
}
