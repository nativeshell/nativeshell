use std::rc::{Rc, Weak};

use crate::{
    codec::Value,
    shell::{
        api_model::{
            DragEffect, DragRequest, PopupMenuRequest, PopupMenuResponse, WindowGeometry,
            WindowGeometryFlags, WindowGeometryRequest, WindowStyle,
        },
        Context, PlatformWindowDelegate,
    },
};

use super::{
    engine::PlatformEngine,
    error::{PlatformError, PlatformResult},
    menu::PlatformMenu,
};

pub type PlatformWindowType = isize;

pub struct PlatformWindow {}

#[allow(unused_variables)]
impl PlatformWindow {
    pub fn new(
        context: Context,
        delegate: Weak<dyn PlatformWindowDelegate>,
        parent: Option<Rc<PlatformWindow>>,
    ) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformWindow>, engine: &PlatformEngine) {}

    pub fn get_platform_window(&self) -> PlatformWindowType {
        Default::default()
    }

    pub fn show(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn ready_to_show(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn close(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn close_with_result(&self, result: Value) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn hide(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn activate(&self) -> PlatformResult<bool> {
        Err(PlatformError::NotImplemented)
    }

    pub fn show_modal<F>(&self, done_callback: F)
    where
        F: FnOnce(PlatformResult<Value>) + 'static,
    {
        done_callback(Err(PlatformError::NotImplemented))
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        Err(PlatformError::NotImplemented)
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        Err(PlatformError::NotImplemented)
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_title(&self, title: String) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn save_position_to_string(&self) -> PlatformResult<String> {
        Ok(String::new())
    }

    pub fn restore_position_from_string(&self, position: String) -> PlatformResult<()> {
        Ok(())
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn begin_drag_session(&self, request: DragRequest) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {}

    pub fn show_popup_menu<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<PopupMenuResponse>) + 'static,
    {
        on_done(Err(PlatformError::NotImplemented))
    }

    pub fn hide_popup_menu(&self, menu: Rc<PlatformMenu>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn show_system_menu(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_window_menu(&self, menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
