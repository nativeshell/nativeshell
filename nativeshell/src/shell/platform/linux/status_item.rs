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
    pub fn assign_weak_self(&self, _weak: Weak<PlatformStatusItem>) {}

    pub fn set_image(&self, _image: Vec<ImageData>) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn set_hint(&self, _hint: String) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn show_menu<F>(&self, _menu: Rc<PlatformMenu>, _offset: Point, on_done: F)
    where
        F: FnOnce(PlatformResult<()>) + 'static,
    {
        on_done(Err(PlatformError::NotAvailable))
    }

    pub fn set_highlighted(&self, _highlighted: bool) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn get_geometry(&self) -> PlatformResult<Rect> {
        Err(PlatformError::NotAvailable)
    }

    pub fn get_screen_id(&self) -> PlatformResult<i64> {
        Err(PlatformError::NotAvailable)
    }
}

pub struct PlatformStatusItemManager {}

impl PlatformStatusItemManager {
    pub fn new(_context: Context) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformStatusItemManager>) {}

    pub fn create_status_item(
        &self,
        _handle: StatusItemHandle,
        _delegate: Weak<RefCell<dyn StatusItemDelegate>>,
        _engine: EngineHandle,
    ) -> PlatformResult<Rc<PlatformStatusItem>> {
        Err(PlatformError::NotAvailable)
    }

    pub fn unregister_status_item(&self, _item: &Rc<PlatformStatusItem>) {}
}
