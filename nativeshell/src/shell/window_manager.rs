use std::{collections::HashMap, rc::Rc};

use velcro::hash_map;

use crate::{
    codec::{
        value::{from_value, to_value},
        MessageCodec, MessageSender, MethodCallError, StandardMethodCodec, Value,
    },
    util::OkLog,
    Error, Result,
};

use super::{
    api_constants::*,
    platform::window::{PlatformWindow, PlatformWindowType},
    Context, ContextRef, EngineHandle, PlatformWindowDelegate, Window, WindowHandle,
    WindowMethodCall, WindowMethodCallReply, WindowMethodCallResult,
};

pub struct WindowManager {
    context: Context,
    windows: HashMap<WindowHandle, Rc<Window>>,
    next_handle: WindowHandle,
    engine_to_window: HashMap<EngineHandle, WindowHandle>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowCreateRequest {
    parent: WindowHandle,
    init_data: Value,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowCreateResponse {
    window_handle: WindowHandle,
}

impl WindowManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        let context_weak = context.weak();
        context
            .window_method_channel
            .borrow_mut()
            .register_method_handler(channel::win::WINDOW_MANAGER, move |call, reply, engine| {
                if let Some(context) = context_weak.get() {
                    Self::on_method_call(&context, call, reply, engine);
                }
            });

        let context_weak = context.weak();
        context
            .window_method_channel
            .borrow_mut()
            .register_method_handler(channel::win::DRAG_SOURCE, move |call, reply, engine| {
                if let Some(context) = context_weak.get() {
                    Self::on_method_call(&context, call, reply, engine);
                }
            });

        WindowManager {
            context: context.weak(),
            windows: HashMap::new(),
            next_handle: WindowHandle(1),
            engine_to_window: HashMap::new(),
        }
    }

    pub fn create_window(
        &mut self,
        init_data: Value,
        parent: Option<WindowHandle>,
    ) -> Result<WindowHandle> {
        if let Some(context) = self.context.get() {
            let window_handle = self.next_handle;
            self.next_handle.0 += 1;

            let parent_engine = parent
                .and_then(|parent| self.windows.get(&parent))
                .map(|win| win.engine_handle);

            let engine_handle = context
                .engine_manager
                .borrow_mut()
                .create_engine(parent_engine)?;

            self.engine_to_window.insert(engine_handle, window_handle);

            let window = Rc::new(Window::new(
                self.context.clone(),
                window_handle,
                engine_handle,
                init_data,
                parent,
            ));

            window.assign_weak_self(Rc::downgrade(&window));

            let parent_platform_window = parent
                .and_then(|h| self.windows.get(&h))
                .map(|w| w.platform_window.borrow().clone());

            let platform_window = Rc::new(PlatformWindow::new(
                self.context.clone(),
                Rc::downgrade(&(window.clone() as Rc<dyn PlatformWindowDelegate>)),
                parent_platform_window,
            ));

            self.windows.insert(window_handle, window.clone());

            platform_window.assign_weak_self(
                Rc::downgrade(&platform_window),
                &context
                    .engine_manager
                    .borrow()
                    .get_engine(engine_handle)
                    .unwrap()
                    .platform_engine,
            );
            window.platform_window.set(platform_window);

            context
                .engine_manager
                .borrow_mut()
                .launch_engine(engine_handle)?;

            Ok(window_handle)
        } else {
            Err(Error::InvalidContext)
        }
    }

    pub fn get_platform_window(&self, handle: WindowHandle) -> Option<PlatformWindowType> {
        self.windows
            .get(&handle)
            .map(|w| w.platform_window.borrow().get_platform_window())
    }

    pub fn get_engine_for_window(&self, handle: WindowHandle) -> Option<EngineHandle> {
        self.windows.get(&handle).map(|w| w.engine_handle)
    }

    pub(super) fn remove_window(&mut self, window: &Window) {
        if let Some(context) = self.context.get() {
            let engine_handle = window.engine_handle;
            let context_copy = self.context.clone();

            // This is a bit hacky; When engine destroy is triggered from flutter
            // platform task runner, we need to schedule this on next run loop turn otherwise
            // it may cause crashes. This particular hack could be avoided by scheduling
            // every flutter message callback on run loop, but is probably not worth the
            // overhead.
            context
                .run_loop
                .borrow()
                .schedule_now(move || {
                    if let Some(context) = context_copy.get() {
                        context
                            .engine_manager
                            .borrow_mut()
                            .remove_engine(engine_handle)
                            .ok_log();
                    }
                })
                .detach();

            self.windows.remove(&window.window_handle);
        }
    }

    fn on_init(&self, window: WindowHandle) -> Value {
        let all_handles = self.windows.keys().map(|h| Value::I64(h.0));
        let all_handles: Vec<Value> = all_handles.collect();
        let window = self.windows.get(&window).unwrap();
        window.initialized.replace(true);
        let parent = window
            .parent
            .map(|h| h.0.into())
            .unwrap_or_else(|| Value::Null);
        Value::Map(hash_map!(
            "allWindows".into() : all_handles.into(),
            "currentWindow".into() : window.window_handle.0.into(),
            "initData".into(): window.init_data.clone(),
            "parentWindow".into(): parent,
        ))
    }

    fn on_create_window(
        &mut self,
        argument: Value,
        parent: WindowHandle,
    ) -> WindowMethodCallResult {
        self.create_window(argument, Some(parent))
            .map_err(MethodCallError::from)
            .map(|win| to_value(WindowCreateResponse { window_handle: win }).unwrap())
    }

    pub(crate) fn message_sender_for_window(
        &self,
        handle: WindowHandle,
        channel_name: &str,
    ) -> Option<MessageSender<Value>> {
        if let Some(context) = self.context.get() {
            let manager = context.message_manager.borrow();
            self.windows
                .get(&handle)
                .map(|w| manager.get_message_sender(w.engine_handle, channel_name))
        } else {
            None
        }
    }

    fn on_method_call(
        context: &ContextRef,
        call: WindowMethodCall,
        reply: WindowMethodCallReply,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::window_manager::GET_API_VERSION => {
                reply.send(Ok(Value::I64(CURRENT_API_VERSION as i64)));
            }
            method::window_manager::INIT_WINDOW => {
                let window = context
                    .window_manager
                    .borrow()
                    .engine_to_window
                    .get(&engine)
                    .cloned();
                match window {
                    Some(window) => {
                        reply.send(Ok(context.window_manager.borrow().on_init(window)));
                        context
                            .window_method_channel
                            .borrow()
                            .get_message_broadcaster(window, channel::win::WINDOW_MANAGER)
                            .broadcast_message(event::window::INITIALIZE, Value::Null);
                    }
                    None => reply.send(Err(MethodCallError {
                        code: "no-window".into(),
                        message: Some("No window associated with engine".into()),
                        details: Value::Null,
                    })),
                }
            }
            method::window_manager::CREATE_WINDOW => {
                let create_request: WindowCreateRequest = from_value(&call.arguments).unwrap();
                reply.send(
                    context
                        .window_manager
                        .borrow_mut()
                        .on_create_window(create_request.init_data, create_request.parent),
                );
            }
            _ => {
                let window = {
                    context
                        .window_manager
                        .borrow()
                        .windows
                        .get(&call.target_window_handle)
                        .cloned()
                };
                if let Some(window) = window {
                    window.on_message(&call.method, call.arguments, reply);
                } else {
                    reply.send(Err(MethodCallError {
                        code: "no-window".into(),
                        message: Some("Target window not found".into()),
                        details: Value::Null,
                    }));
                }
            }
        }
    }

    pub(crate) fn broadcast_message(&self, message: Value) {
        if let Some(context) = self.context.get() {
            let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
            // we use binary messenger directly to be able to encode the message only once
            let message = codec.encode_message(&message);
            for window in self.windows.values() {
                if !window.initialized.get() {
                    continue;
                }
                let manager = context.engine_manager.borrow();
                let engine = manager.get_engine(window.engine_handle);
                if let Some(engine) = engine {
                    engine
                        .binary_messenger()
                        .post_message(channel::DISPATCHER, &message)
                        .ok_log();
                }
            }
        }
    }
}
