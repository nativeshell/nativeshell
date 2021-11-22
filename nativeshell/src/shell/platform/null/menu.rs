use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use super::error::{PlatformError, PlatformResult};
use crate::shell::{api_model::Menu, Context, MenuDelegate, MenuHandle, MenuManager};

pub struct PlatformMenu {}

#[allow(unused_variables)]
impl PlatformMenu {
    pub fn new(
        context: Context,
        handle: MenuHandle,
        delegate: Weak<RefCell<dyn MenuDelegate>>,
    ) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {}

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}

pub struct PlatformMenuManager {}

impl PlatformMenuManager {
    pub fn new(context: Context) -> Self {
        Self {}
    }

    pub(crate) fn assign_weak_self(&self, _weak_self: Weak<PlatformMenuManager>) {}

    pub fn set_app_menu(&self, menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
