use std::rc::Rc;

use super::{Context, ContextRef, platform::hot_key::PlatformHotKeyManager};

pub struct HotKeyManager {
    context: Context,
    platform_manager: Rc<PlatformHotKeyManager>,
}

impl HotKeyManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        let platform_manager = Rc::new(PlatformHotKeyManager::new(context.weak()));
        platform_manager.assign_weak_self(Rc::downgrade(&platform_manager));
        Self {
            context: context.weak(),
            platform_manager,
        }
    }
}
