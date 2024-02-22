use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::codec::{
    EngineMethodChannel, EventSender, MessageChannel, MessageReply, MessageSender, MethodCall,
    MethodCallReply, MethodInvoker, StandardMethodCodec, Value,
};

use super::{Context, ContextRef, EngineHandle, EngineManager};

type MessageCallback = dyn Fn(Value, MessageReply<Value>, EngineHandle);
type MethodCallback = dyn Fn(MethodCall<Value>, MethodCallReply<Value>, EngineHandle);

pub struct MessageManager {
    context: Context,

    message_channels: HashMap<EngineHandle, HashMap<String, MessageChannel<Value>>>,
    message_handlers: Rc<RefCell<HashMap<String, Box<MessageCallback>>>>,

    method_channels: HashMap<EngineHandle, HashMap<String, EngineMethodChannel<Value>>>,
    method_handlers: Rc<RefCell<HashMap<String, Box<MethodCallback>>>>,
}

impl MessageManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        Self {
            context: context.weak(),
            message_channels: HashMap::new(),
            message_handlers: Rc::new(RefCell::new(HashMap::new())),
            method_channels: HashMap::new(),
            method_handlers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn register_message_handler<F>(&mut self, channel: &str, callback: F)
    where
        F: Fn(Value, MessageReply<Value>, EngineHandle) + 'static,
    {
        if let Some(context) = self.context.get() {
            if !self
                .message_handlers
                .as_ref()
                .borrow()
                .contains_key(channel)
            {
                // register handlers on engines
                let manager = context.engine_manager.borrow();
                let engines = manager.get_all_engines();
                for engine in engines {
                    self.register_message_channel_for_engine(&manager, engine, channel);
                }
            }

            self.message_handlers
                .as_ref()
                .borrow_mut()
                .insert(channel.into(), Box::new(callback));
        }
    }

    pub fn register_method_handler<F>(&mut self, channel: &str, callback: F)
    where
        F: Fn(MethodCall<Value>, MethodCallReply<Value>, EngineHandle) + 'static,
    {
        if let Some(context) = self.context.get() {
            if !self.method_handlers.as_ref().borrow().contains_key(channel) {
                // register handlers on engines
                let manager = context.engine_manager.borrow();
                let engines = manager.get_all_engines();
                for engine in engines {
                    self.register_method_channel_for_engine(&manager, engine, channel);
                }
            }

            self.method_handlers
                .as_ref()
                .borrow_mut()
                .insert(channel.into(), Box::new(callback));
        }
    }

    pub fn unregister_message_handler(&mut self, channel: &str) {
        self.message_handlers.as_ref().borrow_mut().remove(channel);

        for entry in self.message_channels.values_mut() {
            entry.remove(channel);
        }
    }

    pub fn unregister_method_handler(&mut self, channel: &str) {
        self.method_handlers.as_ref().borrow_mut().remove(channel);

        for entry in self.method_channels.values_mut() {
            entry.remove(channel);
        }
    }

    pub fn get_message_sender(&self, engine: EngineHandle, channel: &str) -> MessageSender<Value> {
        MessageSender::new(
            self.context.clone(),
            engine,
            channel.into(),
            &StandardMethodCodec,
        )
    }

    pub fn get_event_sender(&self, engine: EngineHandle, channel: &str) -> EventSender<Value> {
        EventSender::new(
            self.context.clone(),
            engine,
            channel.into(),
            &StandardMethodCodec,
        )
    }

    pub fn get_method_invoker(&self, engine: EngineHandle, channel: &str) -> MethodInvoker<Value> {
        MethodInvoker::new(
            self.context.clone(),
            engine,
            channel.into(),
            &StandardMethodCodec,
        )
    }

    pub(super) fn engine_created(&mut self, engine_manager: &EngineManager, engine: EngineHandle) {
        let message_keys: Vec<String> = self
            .message_handlers
            .as_ref()
            .borrow()
            .keys()
            .map(|s| s.into())
            .collect();
        for channel in message_keys {
            self.register_message_channel_for_engine(engine_manager, engine, &channel);
        }

        let method_keys: Vec<String> = self
            .method_handlers
            .as_ref()
            .borrow()
            .keys()
            .map(|s| s.into())
            .collect();

        for channel in method_keys {
            self.register_method_channel_for_engine(engine_manager, engine, &channel);
        }
    }

    fn on_message(
        handlers: Rc<RefCell<HashMap<String, Box<MessageCallback>>>>,
        value: Value,
        channel: &str,
        reply: MessageReply<Value>,
        engine: EngineHandle,
    ) {
        let handlers = handlers.as_ref().borrow();
        if let Some(handler) = handlers.get(channel) {
            handler(value, reply, engine);
        }
    }

    fn on_method(
        handlers: Rc<RefCell<HashMap<String, Box<MethodCallback>>>>,
        call: MethodCall<Value>,
        channel: &str,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    ) {
        let handlers = handlers.as_ref().borrow();
        if let Some(handler) = handlers.get(channel) {
            handler(call, reply, engine);
        }
    }

    fn register_message_channel_for_engine(
        &mut self,
        engine_manager: &EngineManager,
        engine: EngineHandle,
        channel: &str,
    ) {
        let channel_str = String::from(channel);
        let handlers = self.message_handlers.clone();
        let message_channel = MessageChannel::new_with_engine_manager(
            self.context.clone(),
            engine,
            channel,
            &StandardMethodCodec,
            move |value, reply| {
                Self::on_message(handlers.clone(), value, &channel_str, reply, engine);
            },
            engine_manager,
        );
        let map = self.message_channels.entry(engine);
        let entry = map.or_default();
        entry.insert(channel.into(), message_channel);
    }

    fn register_method_channel_for_engine(
        &mut self,
        engine_manager: &EngineManager,
        engine: EngineHandle,
        channel: &str,
    ) {
        let channel_str = String::from(channel);
        let handlers = self.method_handlers.clone();
        let method_channel = EngineMethodChannel::new_with_engine_manager(
            self.context.clone(),
            engine,
            channel,
            &StandardMethodCodec,
            move |call, reply| {
                Self::on_method(handlers.clone(), call, &channel_str, reply, engine);
            },
            engine_manager,
        );
        let map = self.method_channels.entry(engine);
        let entry = map.or_default();
        entry.insert(channel.into(), method_channel);
    }
}
