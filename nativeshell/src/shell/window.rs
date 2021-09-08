use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use crate::{
    codec::{
        value::{from_value, to_value},
        Value,
    },
    util::{LateRefCell, OkLog},
    Error, Result,
};

use super::{
    api_constants::*,
    api_model::{
        DragEffect, DragRequest, DragResult, DraggingInfo, HidePopupMenuRequest, PopupMenuRequest,
        PopupMenuResponse, SetMenuRequest, WindowGeometry, WindowGeometryFlags,
        WindowGeometryRequest, WindowStyle,
    },
    platform::window::PlatformWindow,
    Context, EngineHandle, MenuDelegate, WindowMethodCallReply, WindowMethodCallResult,
    WindowMethodInvoker,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct WindowHandle(pub(super) i64);

pub(super) struct Window {
    context: Context,
    pub(super) window_handle: WindowHandle,
    pub(super) engine_handle: EngineHandle,
    pub(super) platform_window: LateRefCell<Rc<PlatformWindow>>,
    pub(super) init_data: Value,
    pub(super) parent: Option<WindowHandle>,
    pub(super) initialized: Cell<bool>,
    weak_self: LateRefCell<Weak<Self>>,
}

impl Window {
    pub(crate) fn new(
        context: Context,
        window_handle: WindowHandle,
        engine_handle: EngineHandle,
        init_data: Value,
        parent: Option<WindowHandle>,
    ) -> Self {
        Self {
            context,
            window_handle,
            engine_handle,
            platform_window: LateRefCell::new(),
            init_data,
            parent,
            initialized: Cell::new(false),
            weak_self: LateRefCell::new(),
        }
    }

    pub(crate) fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    // fn invoke_method<F>(&self, method: &str, arg: Value, reply: F)
    // where
    //     F: FnOnce(Result<Value, PlatformError>) + 'static,
    // {
    //     let message = encode_method(&self.window_handle, method, arg);
    //     self.message_sender.send_message(&message, |r| {
    //         reply(decode_result(r));
    //     });
    // }

    fn broadcast_message(&self, message: &str, arguments: Value) {
        if let Some(context) = self.context.get() {
            let broadcaster = context
                .window_method_channel
                .borrow()
                .get_message_broadcaster(self.window_handle, channel::win::WINDOW_MANAGER);
            broadcaster.broadcast_message(message, arguments);
        }
    }

    fn drop_target_invoker(&self) -> Option<WindowMethodInvoker> {
        if let Some(context) = self.context.get() {
            context.window_method_channel.borrow().get_method_invoker(
                &context.window_manager.borrow(),
                self.window_handle,
                channel::win::DROP_TARGET,
            )
        } else {
            None
        }
    }

    fn drag_source_invoker(&self) -> Option<WindowMethodInvoker> {
        if let Some(context) = self.context.get() {
            context.window_method_channel.borrow().get_method_invoker(
                &context.window_manager.borrow(),
                self.window_handle,
                channel::win::DRAG_SOURCE,
            )
        } else {
            None
        }
    }

    fn platform_window(&self) -> Rc<PlatformWindow> {
        self.platform_window.borrow().clone()
    }

    fn show(&self) -> Result<()> {
        self.platform_window().show().map_err(|e| e.into())
    }

    fn ready_to_show(&self) -> Result<()> {
        self.platform_window().ready_to_show().map_err(|e| e.into())
    }

    fn close(&self) -> Result<()> {
        self.platform_window().close().map_err(|e| e.into())
    }

    fn close_with_result(&self, result: Value) -> Result<()> {
        self.platform_window()
            .close_with_result(result)
            .map_err(|e| e.into())
    }

    fn hide(&self) -> Result<()> {
        self.platform_window().hide().map_err(|e| e.into())
    }

    fn activate(&self) -> Result<bool> {
        self.platform_window().activate().map_err(|e| e.into())
    }

    fn set_geometry(&self, geometry: WindowGeometryRequest) -> Result<WindowGeometryFlags> {
        self.platform_window()
            .set_geometry(geometry)
            .map_err(|e| e.into())
    }

    fn get_geometry(&self) -> Result<WindowGeometry> {
        self.platform_window().get_geometry().map_err(|e| e.into())
    }

    fn supported_geometry(&self) -> Result<WindowGeometryFlags> {
        self.platform_window()
            .supported_geometry()
            .map_err(|e| e.into())
    }

    fn set_style(&self, style: WindowStyle) -> Result<()> {
        self.platform_window()
            .set_style(style)
            .map_err(|e| e.into())
    }

    fn set_title(&self, title: String) -> Result<()> {
        self.platform_window()
            .set_title(title)
            .map_err(|e| e.into())
    }

    fn save_position_to_string(&self) -> Result<String> {
        self.platform_window()
            .save_position_to_string()
            .map_err(|e| e.into())
    }

    fn restore_position_from_string(&self, position: String) -> Result<()> {
        self.platform_window()
            .restore_position_from_string(position)
            .map_err(|e| e.into())
    }

    fn perform_window_drag(&self) -> Result<()> {
        self.platform_window()
            .perform_window_drag()
            .map_err(|e| e.into())
    }

    fn begin_drag_session(&self, request: DragRequest) -> Result<()> {
        self.platform_window()
            .begin_drag_session(request)
            .map_err(|e| e.into())
    }

    fn show_popup_menu<F>(&self, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(Result<PopupMenuResponse>) + 'static,
    {
        if let Some(context) = self.context.get() {
            let menu = context
                .menu_manager
                .borrow()
                .borrow()
                .get_platform_menu(request.handle);
            match menu {
                Ok(menu) => self
                    .platform_window()
                    .show_popup_menu(menu, request, |r| on_done(r.map_err(|e| e.into()))),
                Err(error) => on_done(Err(error)),
            }
        }
    }

    fn hide_popup_menu(&self, request: HidePopupMenuRequest) -> Result<()> {
        if let Some(context) = self.context.get() {
            let menu = context
                .menu_manager
                .borrow()
                .borrow()
                .get_platform_menu(request.handle)?;
            self.platform_window()
                .hide_popup_menu(menu)
                .map_err(|e| e.into())
        } else {
            Err(Error::InvalidContext)
        }
    }

    fn show_system_menu(&self) -> Result<()> {
        self.platform_window()
            .show_system_menu()
            .map_err(|e| e.into())
    }

    fn set_window_menu(&self, request: SetMenuRequest) -> Result<()> {
        if let Some(context) = self.context.get() {
            match request.handle {
                Some(handle) => {
                    let menu = context
                        .menu_manager
                        .borrow()
                        .borrow()
                        .get_platform_menu(handle)?;
                    self.platform_window()
                        .set_window_menu(Some(menu))
                        .map_err(|e| e.into())
                }
                None => self
                    .platform_window()
                    .set_window_menu(None)
                    .map_err(|e| e.into()),
            }
        } else {
            Err(Error::InvalidContext)
        }
    }

    fn map_result<T>(result: Result<T>) -> WindowMethodCallResult
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }

    fn reply<'a, T, F, A>(reply: WindowMethodCallReply, arg: &'a Value, c: F)
    where
        F: FnOnce(A) -> Result<T>,
        T: serde::Serialize,
        A: serde::Deserialize<'a>,
    {
        let a: std::result::Result<A, _> = from_value(arg);
        match a {
            Ok(a) => {
                let res = c(a);
                let res = Self::map_result(res);
                reply.send(res);
            }
            Err(err) => {
                reply.send(Self::map_result::<()>(Err(err.into())));
            }
        }
    }

    pub(super) fn on_message(&self, method: &str, arg: Value, reply: WindowMethodCallReply) {
        match method {
            method::window::SHOW => {
                return Self::reply(reply, &arg, |()| self.show());
            }
            method::window::SHOW_MODAL => {
                return self.platform_window().show_modal(move |result| {
                    reply.send(Self::map_result(result.map_err(|e| e.into())))
                });
            }
            method::window::READY_TO_SHOW => {
                return Self::reply(reply, &arg, |()| self.ready_to_show());
            }
            method::window::CLOSE => {
                return Self::reply(reply, &arg, |()| self.close());
            }
            method::window::CLOSE_WITH_RESULT => {
                return Self::reply(reply, &arg, |arg| self.close_with_result(arg));
            }
            method::window::HIDE => {
                return Self::reply(reply, &arg, |()| self.hide());
            }
            method::window::ACTIVATE => {
                return Self::reply(reply, &arg, |()| self.activate());
            }
            method::window::SET_GEOMETRY => {
                return Self::reply(reply, &arg, |geometry| self.set_geometry(geometry));
            }
            method::window::GET_GEOMETRY => {
                return Self::reply(reply, &arg, |()| self.get_geometry());
            }
            method::window::SUPPORTED_GEOMETRY => {
                return Self::reply(reply, &arg, |()| self.supported_geometry());
            }
            method::window::SET_STYLE => {
                return Self::reply(reply, &arg, |style| self.set_style(style));
            }
            method::window::SET_TITLE => {
                return Self::reply(reply, &arg, |title| self.set_title(title));
            }
            method::window::SAVE_POSITION_TO_STRING => {
                return Self::reply(reply, &arg, |()| self.save_position_to_string());
            }
            method::window::RESTORE_POSITION_FROM_STRING => {
                return Self::reply(reply, &arg, |position: String| {
                    self.restore_position_from_string(position)
                });
            }
            method::window::PERFORM_WINDOW_DRAG => {
                return Self::reply(reply, &arg, |()| self.perform_window_drag());
            }
            method::window::SHOW_POPUP_MENU => {
                let request: std::result::Result<PopupMenuRequest, _> = from_value(&arg);
                match request {
                    Ok(request) => {
                        return self
                            .show_popup_menu(request, move |res| reply.send(Self::map_result(res)))
                    }
                    Err(err) => return reply.send(Self::map_result::<()>(Err(err.into()))),
                }
            }
            method::window::HIDE_POPUP_MENU => {
                return Self::reply(reply, &arg, |req| self.hide_popup_menu(req));
            }
            method::window::SHOW_SYSTEM_MENU => {
                return Self::reply(reply, &arg, |()| self.show_system_menu());
            }
            method::window::SET_WINDOW_MENU => {
                return Self::reply(reply, &arg, |req| self.set_window_menu(req));
            }
            method::drag_source::BEGIN_DRAG_SESSION => {
                return Self::reply(reply, &arg, |request| self.begin_drag_session(request));
            }
            _ => {}
        }

        reply.send(Ok(Value::Null));
    }
}

pub trait PlatformWindowDelegate {
    fn visibility_changed(&self, visible: bool);
    fn did_request_close(&self);
    fn will_close(&self);

    fn dragging_exited(&self);
    fn dragging_updated(&self, info: &DraggingInfo);
    fn perform_drop(&self, info: &DraggingInfo);

    fn drag_ended(&self, effect: DragEffect);

    fn get_engine_handle(&self) -> EngineHandle;
}

impl PlatformWindowDelegate for Window {
    fn visibility_changed(&self, visible: bool) {
        self.broadcast_message(event::window::VISIBILITY_CHANGED, Value::Bool(visible));
    }

    fn did_request_close(&self) {
        self.broadcast_message(event::window::CLOSE_REQUEST, Value::Null);
    }

    fn will_close(&self) {
        if let Some(context) = self.context.get() {
            self.broadcast_message(event::window::CLOSE, Value::Null);
            context.window_manager.borrow_mut().remove_window(self);
        }
    }

    fn dragging_exited(&self) {
        if let Some(invoker) = self.drop_target_invoker() {
            invoker
                .call_method(method::drag_driver::DRAGGING_EXITED, Value::Null, |_| {})
                .ok_log();
        }
    }

    fn dragging_updated(&self, info: &DraggingInfo) {
        let weak = self.weak_self.clone_value();
        if let Some(invoker) = self.drop_target_invoker() {
            invoker
                .call_method(
                    method::drag_driver::DRAGGING_UPDATED,
                    to_value(info).unwrap(),
                    move |r| {
                        let s = weak.upgrade();
                        if let (Ok(result), Some(s)) = (r, s) {
                            let result: DragResult =
                                from_value(&result).ok_log().unwrap_or(DragResult {
                                    effect: DragEffect::None,
                                });
                            s.platform_window().set_pending_effect(result.effect);
                        }
                    },
                )
                .ok_log();
        }
    }

    fn perform_drop(&self, info: &DraggingInfo) {
        if let Some(invoker) = self.drop_target_invoker() {
            invoker
                .call_method(
                    method::drag_driver::PERFORM_DROP,
                    to_value(info).unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }

    fn drag_ended(&self, effect: DragEffect) {
        if let Some(invoker) = self.drag_source_invoker() {
            invoker
                .call_method(
                    method::drag_source::DRAG_SESSION_ENDED,
                    to_value(effect).unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }

    fn get_engine_handle(&self) -> EngineHandle {
        self.engine_handle
    }
}
