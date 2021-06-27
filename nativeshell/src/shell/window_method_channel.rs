use std::{cell::RefCell, collections::HashMap, rc::Rc};

use velcro::hash_map;

use crate::{
    codec::{MessageReply, MessageSender, MethodCallError, Value},
    Result,
};

use super::{
    api_constants::channel, Context, ContextRef, EngineHandle, WindowHandle, WindowManager,
};

pub struct WindowMethodChannel {
    context: Context,
    handlers: Rc<RefCell<HashMap<String, Box<WindowMethodCallback>>>>,
}

pub struct WindowMethodCall {
    pub target_window_handle: WindowHandle,
    pub method: String,
    pub channel: String,
    pub arguments: Value,
}

pub struct WindowMethodCallReply {
    reply: MessageReply<Value>,
}

pub type WindowMethodCallResult = std::result::Result<Value, MethodCallError<Value>>;

impl WindowMethodCallReply {
    pub fn send(self, result: WindowMethodCallResult) {
        self.reply.send(encode_result(result));
    }
}

#[derive(Clone)]
pub struct WindowMethodInvoker {
    sender: MessageSender<Value>,
    channel_name: String,
    target_window_handle: WindowHandle,
}

#[derive(Clone)]
pub struct WindowMessageBroadcaster {
    context: Context,
    channel: String,
    source_window_handle: WindowHandle,
}

impl WindowMessageBroadcaster {
    pub fn broadcast_message(&self, message: &str, arguments: Value) {
        let encoded = encode_message(
            &self.source_window_handle,
            &self.channel,
            message,
            arguments,
        );
        if let Some(context) = self.context.get() {
            context.window_manager.borrow().broadcast_message(encoded);
        }
    }
}

impl WindowMethodInvoker {
    pub fn call_method<F>(&self, method: &str, arguments: Value, reply: F) -> Result<()>
    where
        F: FnOnce(WindowMethodCallResult) + 'static,
    {
        self.sender.send_message(
            &encode_method_call(WindowMethodCall {
                target_window_handle: self.target_window_handle,
                method: method.into(),
                channel: self.channel_name.clone(),
                arguments,
            }),
            move |value| reply(decode_result(value)),
        )
    }
}

type WindowMethodCallback = dyn Fn(WindowMethodCall, WindowMethodCallReply, EngineHandle);

impl WindowMethodChannel {
    pub(super) fn new(context: &ContextRef) -> Self {
        let handlers = Rc::new(RefCell::new(HashMap::new()));

        let context_weak = context.weak();
        let handlers_copy = handlers.clone();
        context
            .message_manager
            .borrow_mut()
            .register_message_handler(
                channel::DISPATCHER, //
                move |message, reply, engine| {
                    if let Some(context) = context_weak.get() {
                        Self::on_message(&context, handlers_copy.clone(), message, reply, engine);
                    }
                },
            );
        Self {
            context: context.weak(),
            handlers,
        }
    }

    pub fn register_method_handler<F>(&mut self, channel: &str, callback: F)
    where
        F: Fn(WindowMethodCall, WindowMethodCallReply, EngineHandle) + 'static,
    {
        self.handlers
            .as_ref()
            .borrow_mut()
            .insert(channel.into(), Box::new(callback));
    }

    pub fn unregister_method_handler(&mut self, channel: &str) {
        self.handlers.as_ref().borrow_mut().remove(channel);
    }

    pub fn get_method_invoker(
        &self,
        window_manager: &WindowManager,
        window: WindowHandle,
        channel_name: &str,
    ) -> Option<WindowMethodInvoker> {
        window_manager
            .message_sender_for_window(window, channel::DISPATCHER)
            .map(|sender| WindowMethodInvoker {
                sender,
                channel_name: channel_name.into(),
                target_window_handle: window,
            })
    }

    pub fn get_message_broadcaster(
        &self,
        source_window: WindowHandle,
        channel_name: &str,
    ) -> WindowMessageBroadcaster {
        WindowMessageBroadcaster {
            context: self.context.clone(),
            channel: channel_name.into(),
            source_window_handle: source_window,
        }
    }

    fn on_message(
        context: &ContextRef,
        handlers: Rc<RefCell<HashMap<String, Box<WindowMethodCallback>>>>,
        message: Value,
        reply: MessageReply<Value>,
        engine: EngineHandle,
    ) {
        let call = decode_method_call(message);
        let handlers = handlers.as_ref().borrow();
        let handler = handlers.get(&call.channel);
        match handler {
            // found handler for message
            Some(handler) => {
                handler(call, WindowMethodCallReply { reply }, engine);
            }
            // no handler, forward message to target window
            None => {
                let sender = context
                    .window_manager
                    .borrow()
                    .message_sender_for_window(call.target_window_handle, channel::DISPATCHER);
                if let Some(sender) = sender {
                    sender
                        .send_message(&encode_method_call(call), |reply_in| reply.send(reply_in))
                        .ok();
                }
            }
        }
    }
}

impl Drop for WindowMethodChannel {
    fn drop(&mut self) {
        if let Some(context) = self.context.get() {
            context
                .message_manager
                .borrow_mut()
                .unregister_message_handler(channel::DISPATCHER);
        }
    }
}

fn encode_method_call(call: WindowMethodCall) -> Value {
    Value::Map(hash_map! {
        "targetWindowHandle".into() : call.target_window_handle.0.into(),
        "method".into() : call.method.into(),
        "channel".into() : call.channel.into(),
        "arguments".into() : call.arguments,
    })
}

fn encode_message(handle: &WindowHandle, channel: &str, message: &str, arguments: Value) -> Value {
    Value::Map(hash_map! {
        "sourceWindowHandle".into() : handle.0.into(),
        "message".into() : message.into(),
        "channel".into() : channel.into(),
        "arguments".into() : arguments,
    })
}

fn decode_method_call(call: Value) -> WindowMethodCall {
    if let Value::Map(mut map) = call {
        let target_window_handle = map.remove(&"targetWindowHandle".into());
        let method = map.remove(&"method".into());
        let channel = map.remove(&"channel".into());
        let arguments = map.remove(&"arguments".into());

        match (target_window_handle, method, channel) {
            (
                Some(Value::I64(target_window_handle)),
                Some(Value::String(method)),
                Some(Value::String(channel)),
            ) => WindowMethodCall {
                target_window_handle: WindowHandle(target_window_handle),
                method,
                channel,
                arguments: arguments.unwrap_or(Value::Null),
            },
            _ => {
                panic!("Invalid method call")
            }
        }
    } else {
        panic!("Invalid method call");
    }
}

fn decode_result(result: Value) -> WindowMethodCallResult {
    if let Value::Map(mut map) = result {
        let code = map.remove(&"code".into());
        let message = map.remove(&"message".into());
        let details = map.remove(&"details".into());
        let result = map.remove(&"result".into());
        match (code, message, details, result) {
            (Some(Value::String(code)), None, details, None) => Err(MethodCallError {
                code,
                message: None,
                details: details.unwrap_or(Value::Null),
            }),
            (Some(Value::String(code)), Some(Value::String(message)), details, None) => {
                Err(MethodCallError {
                    code,
                    message: Some(message),
                    details: details.unwrap_or(Value::Null),
                })
            }
            (None, None, None, Some(value)) => Ok(value),
            (_, _, _, _) => panic!("Invalid value"),
        }
    } else {
        panic!("Invalid value");
    }
}

fn encode_error(code: &str, message: Option<&str>, details: Value) -> Value {
    let message = match message {
        Some(message) => message.into(),
        None => Value::Null,
    };
    Value::Map(hash_map! {
        "code".into() : code.into(),
        "message".into() : message,
        "details".into() : details,
    })
}

fn encode_result(result: WindowMethodCallResult) -> Value {
    match result {
        Ok(value) => Value::Map(hash_map! {
            "result".into() : value,
        }),
        Err(error) => encode_error(&error.code, error.message.as_deref(), error.details),
    }
}
