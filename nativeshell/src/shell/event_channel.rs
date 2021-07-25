use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{codec::Value, Error, Result};

use super::{Context, EngineHandle, MethodCallHandler, RegisteredMethodCallHandler};

pub struct EventSink {
    context: Context,
    id: i64,
    channel_name: String,
    engine_handle: EngineHandle,
}

impl EventSink {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn send_message(&self, message: &Value) -> Result<()> {
        if let Some(context) = self.context.get() {
            context
                .message_manager
                .borrow()
                .get_event_sender(self.engine_handle, &self.channel_name)
                .send_event(message)
        } else {
            Err(Error::InvalidContext)
        }
    }
}

pub trait EventChannelHandler: Sized + 'static {
    // Implementation can store weak reference if it needs to pass it around.
    // Guaranteed to call before any other methods.
    fn assign_weak_self(&mut self, _weak_self: Weak<RefCell<Self>>) {}

    // Implementation can store the event sink and use it to send event messages.
    fn register_event_sink(&mut self, sink: EventSink, listen_argument: Value);

    // Called when event sink has either been unregistered or engine stopped.
    fn unregister_event_sink(&mut self, sink_id: i64);

    // Registers itself for handling even sink registration methods.
    fn register(self, context: Context, channel: &str) -> RegisteredEventChannel<Self> {
        RegisteredEventChannel::new(context, channel, self)
    }
}

pub struct RegisteredEventChannel<T: EventChannelHandler> {
    _internal: RegisteredMethodCallHandler<EventChannelInternal<T>>,
    handler: Rc<RefCell<T>>,
}

impl<T: EventChannelHandler> RegisteredEventChannel<T> {
    pub fn new(context: Context, channel: &str, handler: T) -> Self {
        Self::new_ref(context, channel, Rc::new(RefCell::new(handler)))
    }

    pub fn new_ref(context: Context, channel: &str, handler: Rc<RefCell<T>>) -> Self {
        handler
            .borrow_mut()
            .assign_weak_self(Rc::downgrade(&handler));

        Self {
            _internal: EventChannelInternal {
                context: context.clone(),
                handler: handler.clone(),
                channel_name: channel.into(),
                next_sink_id: 1,
                engine_to_sink: HashMap::new(),
            }
            .register(context, channel),
            handler,
        }
    }

    pub fn borrow(&self) -> Ref<T> {
        self.handler.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.handler.borrow_mut()
    }
}

//
// Internal
//

struct EventChannelInternal<T: EventChannelHandler> {
    context: Context,
    channel_name: String,
    pub handler: Rc<RefCell<T>>,
    next_sink_id: i64,
    engine_to_sink: HashMap<EngineHandle, i64>,
}

impl<T: EventChannelHandler> MethodCallHandler for EventChannelInternal<T> {
    fn on_method_call(
        &mut self,
        call: crate::codec::MethodCall<Value>,
        reply: crate::codec::MethodCallReply<Value>,
        engine: super::EngineHandle,
    ) {
        match call.method.as_str() {
            "listen" => {
                let sink_id = self.next_sink_id;
                self.next_sink_id += 1;
                let sink = EventSink {
                    context: self.context.clone(),
                    id: sink_id,
                    channel_name: self.channel_name.clone(),
                    engine_handle: engine,
                };
                self.handler
                    .borrow_mut()
                    .register_event_sink(sink, call.args);
                reply.send_ok(Value::Null);
            }
            "cancel" => {
                if let Some(sink_id) = self.engine_to_sink.remove(&engine) {
                    self.handler.borrow_mut().unregister_event_sink(sink_id);
                }
                reply.send_ok(Value::Null);
            }
            _ => {}
        }
    }

    // Called when engine is about to be destroyed.
    fn on_engine_destroyed(&mut self, engine: EngineHandle) {
        if let Some(sink_id) = self.engine_to_sink.remove(&engine) {
            self.handler.borrow_mut().unregister_event_sink(sink_id);
        }
    }
}
