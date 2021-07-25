use std::marker::PhantomData;

use crate::shell::{BinaryMessengerReply, Context, ContextRef, EngineHandle, EngineManager};

use super::MessageCodec;

pub struct MessageChannel<V>
where
    V: 'static,
{
    context: Context,
    channel_name: String,
    engine_handle: EngineHandle,
    _data: PhantomData<V>,
}

impl<V> MessageChannel<V> {
    pub fn new<F>(
        context: &ContextRef,
        engine_handle: EngineHandle,
        channel_name: &str,
        codec: &'static dyn MessageCodec<V>,
        callback: F,
    ) -> Self
    where
        F: Fn(V, MessageReply<V>) + 'static,
    {
        Self::new_with_engine_manager(
            context.weak(),
            engine_handle,
            channel_name,
            codec,
            callback,
            &context.engine_manager.borrow(),
        )
    }

    pub fn new_with_engine_manager<F>(
        context: Context,
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
            context,
            channel_name: channel_name.into(),
            engine_handle,
            _data: PhantomData {},
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
        if let Some(context) = self.context.get() {
            let engine_manager = context.engine_manager.borrow();
            let engine = engine_manager.get_engine(self.engine_handle);
            if let Some(engine) = engine {
                engine
                    .binary_messenger()
                    .unregister_channel_handler(&self.channel_name);
            }
        }
    }
}
