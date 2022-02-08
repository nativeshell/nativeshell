use once_cell::sync::OnceCell;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashMap,
    convert::TryInto,
    fmt::Display,
    rc::Rc,
    sync::atomic::AtomicI64,
};

use crate::{
    message_channel::codec::Serializer, raw, Context, DartPort, DartValue, IsolateId, NativePort,
    RunLoopSender, Value,
};

use super::codec::Deserializer;

#[derive(Debug)]
pub enum SendMessageError {
    InvalidIsolate,
    MessageRefused,
    IsolateShutDown,
    ChannelNotFound { channel: String },
    HandlerNotRegistered { channel: String },
}

impl Display for SendMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendMessageError::InvalidIsolate => write!(f, "target isolate not found"),
            SendMessageError::MessageRefused => write!(f, "target isolate refused the message"),
            SendMessageError::IsolateShutDown => {
                write!(f, "target isolate was shut down while waiting for response")
            }
            SendMessageError::ChannelNotFound { channel } => {
                write!(f, "message channel \"{}\" not found", channel)
            }
            SendMessageError::HandlerNotRegistered { channel } => {
                write!(
                    f,
                    "message handler for channel \"{}\" not registered",
                    channel
                )
            }
        }
    }
}

impl std::error::Error for SendMessageError {}

pub trait MessageChannelDelegate {
    fn on_isolate_joined(&self, isolate: IsolateId);
    fn on_message(&self, isolate: IsolateId, message: Value, reply: Box<dyn FnOnce(Value) -> bool>);
    fn on_isolate_exited(&self, isolate: IsolateId);
}

pub struct MessageChannel {
    // used to get isolate exit notification
    native_port: RefCell<Option<NativePort>>,
    isolates: RefCell<HashMap<IsolateId, DartPort>>,
    delegates: RefCell<HashMap<String, Rc<dyn MessageChannelDelegate>>>,
    pending_replies: RefCell<HashMap<i64, PendingReply>>,
    next_message_id: Cell<i64>,
}

struct PendingReply {
    reply: Box<dyn FnOnce(Result<Value, SendMessageError>)>,
    isolate_id: IsolateId,
}

impl MessageChannel {
    fn new() -> Self {
        RUN_LOOP_SENDER
            .set(Context::get().run_loop().new_sender())
            .map_err(|_| ())
            .expect("Message channel already initialized");
        Self {
            native_port: RefCell::new(None),
            isolates: RefCell::new(HashMap::new()),
            delegates: RefCell::new(HashMap::new()),
            pending_replies: RefCell::new(HashMap::new()),
            next_message_id: Cell::new(0),
        }
    }

    pub fn send_message<F>(
        &self,
        target_isolate: IsolateId,
        channel: &str,
        message: Value,
        reply: F,
    ) where
        F: FnOnce(Result<Value, SendMessageError>) + 'static,
    {
        let isolate = self.isolates.borrow().get(&target_isolate).cloned();
        if let Some(isolate) = isolate {
            let id = self.next_message_id.replace(self.next_message_id.get() + 1);
            self.pending_replies.borrow_mut().insert(
                id,
                PendingReply {
                    reply: Box::new(reply),
                    isolate_id: target_isolate,
                },
            );
            let v = Serializer::serialize(
                vec![
                    Value::String("message".into()),
                    channel.into(),
                    id.into(),
                    message,
                ]
                .into(),
            );

            if !isolate.send(DartValue::Array(v)) {
                let reply = self.pending_replies.borrow_mut().remove(&id);
                if let Some(reply) = reply {
                    (reply.reply)(Err(SendMessageError::MessageRefused));
                }
            }
        } else {
            reply(Err(SendMessageError::InvalidIsolate));
        }
    }

    pub fn register_delegate<F>(&self, channel: &str, delegate: Rc<F>)
    where
        F: MessageChannelDelegate + 'static,
    {
        self.delegates.borrow_mut().insert(channel.into(), delegate);
    }

    pub fn unregister_delegate(&self, channel: &str) {
        self.delegates.borrow_mut().remove(channel);
    }

    fn all_delegates(&self) -> Vec<Rc<dyn MessageChannelDelegate>> {
        self.delegates.borrow().values().cloned().collect()
    }

    fn register_isolate(&self, isolate_id: i64, port: raw::DartPort) {
        // Initialize native port if we need to
        let native_port = self
            .native_port
            .borrow_mut()
            .get_or_insert_with(|| {
                NativePort::new("MessageChannelPort", |_, v| {
                    let sender = RUN_LOOP_SENDER.get().unwrap();
                    sender.send(move || {
                        Context::get()
                            .message_channel()
                            .on_nativeport_value_received(v);
                    });
                })
            })
            .as_send_port();

        // send native port to dart
        let isolate_port = DartPort::new(port);
        isolate_port.send(native_port);
        self.isolates.borrow_mut().insert(isolate_id, isolate_port);

        for d in self.all_delegates() {
            d.on_isolate_joined(isolate_id);
        }
    }

    fn on_value_received(&self, isolate_id: IsolateId, value: Value) {
        if self.handle_message(isolate_id, value).is_none() {
            panic!("MessageChannel: Malformed message");
        }
    }

    fn handle_message(&self, isolate_id: IsolateId, value: Value) -> Option<()> {
        let value: Vec<Value> = value.try_into().ok()?;
        let mut iter = value.into_iter();
        let message: String = iter.next()?.try_into().ok()?;
        match message.as_ref() {
            "no_channel" => {
                self.handle_no_channel(iter.next()?.try_into().ok()?, iter.next()?.try_into().ok()?)
            }
            "no_handler" => {
                self.handle_no_handler(iter.next()?.try_into().ok()?, iter.next()?.try_into().ok()?)
            }
            "reply" => {
                self.handle_reply(iter.next()?.try_into().ok()?, iter.next()?);
            }
            "message" => {
                let reply_id: i64 = iter.next()?.try_into().ok()?;
                let channel: String = iter.next()?.try_into().ok()?;
                let message = iter.next()?;
                self.handle_send_message(isolate_id, channel, reply_id, message);
            }
            _ => {}
        }
        Some(())
    }

    fn handle_no_channel(&self, reply_id: i64, channel: String) {
        if let Some(reply) = self.pending_replies.borrow_mut().remove(&reply_id) {
            (reply.reply)(Err(SendMessageError::ChannelNotFound { channel }));
        }
    }

    fn handle_no_handler(&self, reply_id: i64, channel: String) {
        if let Some(reply) = self.pending_replies.borrow_mut().remove(&reply_id) {
            (reply.reply)(Err(SendMessageError::HandlerNotRegistered { channel }));
        }
    }

    fn handle_reply(&self, reply_id: i64, value: Value) {
        if let Some(reply) = self.pending_replies.borrow_mut().remove(&reply_id) {
            (reply.reply)(Ok(value));
        }
    }

    fn handle_send_message(&self, isolate_id: i64, channel: String, reply_id: i64, message: Value) {
        let delegate = self.delegates.borrow().get(&channel).cloned();
        let port = self
            .isolates
            .borrow()
            .get(&isolate_id)
            .cloned()
            .expect("received message from unknown isolate");
        match delegate {
            Some(delegate) => {
                let reply = Box::new(move |value: Value| {
                    let v = Serializer::serialize(
                        vec![Value::String("reply".into()), reply_id.into(), value].into(),
                    );
                    port.send(DartValue::Array(v))
                });
                delegate.on_message(isolate_id, message, reply);
            }
            None => {
                let v = Serializer::serialize(
                    vec![
                        Value::String("reply_no_channel".into()),
                        reply_id.into(),
                        channel.into(),
                    ]
                    .into(),
                );
                port.send(DartValue::Array(v));
            }
        }
    }

    fn handle_isolate_exit(&self, isolate_id: IsolateId) {
        self.isolates.borrow_mut().remove(&isolate_id);
        for delegate in self.all_delegates() {
            delegate.on_isolate_exited(isolate_id);
        }
        // TODO(knopp) use drain_filter once stable
        let replies_to_remove: Vec<_> = self
            .pending_replies
            .borrow()
            .iter()
            .filter_map(|(id, reply)| {
                if reply.isolate_id == isolate_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for reply in replies_to_remove {
            if let Some(reply) = self.pending_replies.borrow_mut().remove(&reply) {
                (reply.reply)(Err(SendMessageError::IsolateShutDown));
            }
        }
    }

    // Received value from native port. This is currently used for isolate exit
    // notifications
    fn on_nativeport_value_received(&self, v: DartValue) {
        if let DartValue::Array(value) = v {
            let mut iter = value.into_iter();
            let first = iter.next();
            let second = iter.next();

            if let (Some(DartValue::String(message)), Some(isolate_id)) = (first, second) {
                let isolate_id = match isolate_id {
                    DartValue::I32(id) => id as i64,
                    DartValue::I64(id) => id,
                    id => panic!("invalid isolate id {:?}", id),
                };
                let message = message.to_string_lossy();
                if message == "isolate_exit" {
                    self.handle_isolate_exit(isolate_id);
                }
            }
        }
    }
}

pub trait ContextMessageChannel {
    fn message_channel(&self) -> Ref<MessageChannel>;
}

impl ContextMessageChannel for Context {
    fn message_channel(&self) -> Ref<MessageChannel> {
        self.get_attachment(MessageChannel::new)
    }
}

static RUN_LOOP_SENDER: OnceCell<RunLoopSender> = OnceCell::new();

static NEXT_THREAD_ID: AtomicI64 = AtomicI64::new(1);

// Accepts port, returns isolate id
pub(super) extern "C" fn register_isolate(port: i64) -> i64 {
    if RUN_LOOP_SENDER.get().is_none() {
        return -1;
    }
    let isolate_id = NEXT_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let sender = RUN_LOOP_SENDER.get().unwrap();
    sender.send(move || {
        Context::get()
            .message_channel()
            .register_isolate(isolate_id, port);
    });
    isolate_id
}

pub(super) extern "C" fn post_message(isolate_id: IsolateId, message: *mut u8, len: u64) {
    let vec = unsafe { Vec::from_raw_parts(message, len as usize, len as usize) };
    let value = unsafe { Deserializer::deserialize(&vec) };
    let sender = RUN_LOOP_SENDER.get().unwrap();
    sender.send(move || {
        Context::get()
            .message_channel()
            .on_value_received(isolate_id, value);
    });
}
