use std::rc::{Rc, Weak};

use crate::shell::{api_model::Menu, Context, MenuHandle, MenuManager};

use super::error::{PlatformError, PlatformResult};

pub struct PlatformMenu {}

#[allow(unused_variables)]
impl PlatformMenu {
    pub fn new(context: Rc<Context>, handle: MenuHandle) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {}

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}

pub struct PlatformMenuManager {}

impl PlatformMenuManager {
    pub fn new(context: Rc<Context>) -> Self {
        Self {}
    }

    pub fn set_app_menu(&self, menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
