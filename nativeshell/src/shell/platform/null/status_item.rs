use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    shell::{
        api_model::ImageData,
        status_item_manager::{StatusItemDelegate, StatusItemHandle},
        EngineHandle, Point, Rect,
    },
    Context,
};

use super::{
    error::{PlatformError, PlatformResult},
    menu::PlatformMenu,
};

pub struct PlatformStatusItem {
    pub(crate) engine: EngineHandle,
}

impl PlatformStatusItem {
    pub fn assign_weak_self(&self, weak: Weak<PlatformStatusItem>) {}

    pub fn set_image(&self, image: Vec<ImageData>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_hint(&self, hint: String) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn show_menu<F>(&self, menu: Rc<PlatformMenu>, offset: Point, on_done: F)
    where
        F: FnOnce(PlatformResult<()>) + 'static,
    {
        on_done(Err(PlatformError::NotImplemented))
    }

    pub fn set_highlighted(&self, highlighted: bool) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn get_geometry(&self) -> PlatformResult<Rect> {
        Err(PlatformError::NotImplemented)
    }

    pub fn get_screen_id(&self) -> PlatformResult<i64> {
        Err(PlatformError::NotImplemented)
    }
}

pub struct PlatformStatusItemManager {}

impl PlatformStatusItemManager {
    pub fn new(context: Context) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformStatusItemManager>) {}

    pub fn create_status_item(
        &self,
        handle: StatusItemHandle,
        delegate: Weak<RefCell<dyn StatusItemDelegate>>,
        engine: EngineHandle,
    ) -> PlatformResult<Rc<PlatformStatusItem>> {
        Err(PlatformError::NotImplemented)
    }

    pub fn unregister_status_item(&self, item: &Rc<PlatformStatusItem>) {}
}
