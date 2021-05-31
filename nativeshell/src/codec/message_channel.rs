use std::rc::Rc;

use crate::{
    shell::{BinaryMessengerReply, Context, EngineHandle, EngineManager},
    Error, Result,
};

use super::MessageCodec;

pub struct MessageChannel<V>
where
    V: 'static,
{
    context: Rc<Context>,
    sender: MessageSender<V>,
}

impl<V> MessageChannel<V> {
    pub fn new<F>(
        context: Rc<Context>,
        engine_handle: EngineHandle,
        channel_name: &str,
        codec: &'static dyn MessageCodec<V>,
        callback: F,
    ) -> Self
    where
        F: Fn(V, MessageReply<V>) + 'static,
    {
        Self::new_with_engine_manager(
            context.clone(),
            engine_handle,
            channel_name,
            codec,
            callback,
            &context.engine_manager.borrow(),
        )
    }

    pub fn new_with_engine_manager<F>(
        context: Rc<Context>,
        engine_handle: EngineHandle,
        channel_name: &str,
        codec: &'static dyn MessageCodec<V>,
        callback: F,
        engine_manager: &EngineManager,
    ) -> Self
    where
        F: Fn(V, MessageReply<V>) + 'static,
    {
        let res = MessageChannel {
            context: context.clone(),
            sender: MessageSender {
                context,
                engine_handle,
                channel_name: channel_name.into(),
                codec,
            },
        };

        let engine = engine_manager.get_engine(engine_handle);
        if let Some(engine) = engine {
            let codec = codec;
            engine
                .binary_messenger()
                .register_channel_handler(channel_name, move |data, reply| {
                    let message = codec.decode_message(data).unwrap();
                    let reply = MessageReply { reply, codec };
                    callback(message, reply);
                });
        }
        res
    }

    pub fn sender(&self) -> &MessageSender<V> {
        &self.sender
    }
}

//
//
//

#[derive(Clone)]
pub struct MessageSender<V>
where
    V: 'static,
{
    context: Rc<Context>,
    engine_handle: EngineHandle,
    channel_name: String,
    codec: &'static dyn MessageCodec<V>,
}

impl<V> MessageSender<V> {
    pub fn send_message<F>(&self, message: &V, reply: F) -> Result<()>
    where
        F: FnOnce(V) + 'static,
    {
        let encoded = self.codec.encode_message(message);
        let engine_manager = self.context.engine_manager.borrow();
        let engine = engine_manager.get_engine(self.engine_handle);
        if let Some(engine) = engine {
            let codec = self.codec;
            engine
                .binary_messenger()
                .send_message(&self.channel_name, &encoded, move |message| {
                    let message = codec.decode_message(message).unwrap();
                    reply(message);
                })
        } else {
            Err(Error::InvalidEngineHandle)
        }
    }

    pub fn post_message(&self, message: &V) -> Result<()> {
        let encoded = self.codec.encode_message(message);
        let engine_manager = self.context.engine_manager.borrow();
        let engine = engine_manager.get_engine(self.engine_handle);
        if let Some(engine) = engine {
            engine
                .binary_messenger()
                .post_message(&self.channel_name, &encoded)
        } else {
            Err(Error::InvalidEngineHandle)
        }
    }
}

//
//
//

pub struct MessageReply<V>
where
    V: 'static,
{
    reply: BinaryMessengerReply,
    codec: &'static dyn MessageCodec<V>,
}

impl<V> MessageReply<V> {
    pub fn send(self, value: V) {
        let encoded = self.codec.encode_message(&value);
        self.reply.send(&encoded);
    }
}

impl<V> Drop for MessageChannel<V> {
    fn drop(&mut self) {
        let engine_manager = self.context.engine_manager.borrow();
        let engine = engine_manager.get_engine(self.sender.engine_handle);
        if let Some(engine) = engine {
            engine
                .binary_messenger()
                .unregister_channel_handler(&self.sender.channel_name);
        }
    }
}
