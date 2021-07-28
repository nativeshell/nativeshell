use std::marker::PhantomData;

use crate::shell::{BinaryMessengerReply, Context, ContextRef, EngineHandle, EngineManager};

use super::{MethodCall, MethodCallError, MethodCallResult, MethodCodec};

// Low level interface to method channel on single engine
pub struct EngineMethodChannel<V>
where
    V: 'static,
{
    context: Context,
    channel_name: String,
    engine_handle: EngineHandle,
    _data: PhantomData<V>,
}

impl<V> EngineMethodChannel<V> {
    pub fn new<F>(
        context: ContextRef,
        engine_handle: EngineHandle,
        channel_name: &str,
        codec: &'static dyn MethodCodec<V>,
        callback: F,
    ) -> Self
    where
        F: Fn(MethodCall<V>, MethodCallReply<V>) + 'static,
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
        codec: &'static dyn MethodCodec<V>,
        callback: F,
        engine_manager: &EngineManager,
    ) -> Self
    where
        F: Fn(MethodCall<V>, MethodCallReply<V>) + 'static,
    {
        let res = EngineMethodChannel {
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
                    let message = codec.decode_method_call(data).unwrap();
                    let reply = MethodCallReply { reply, codec };
                    callback(message, reply);
                });
        }
        res
    }
}

pub struct MethodCallReply<V>
where
    V: 'static,
{
    reply: BinaryMessengerReply,
    codec: &'static dyn MethodCodec<V>,
}

impl<V> MethodCallReply<V> {
    pub fn send(self, value: MethodCallResult<V>) {
        let encoded = self.codec.encode_method_call_result(&value);
        self.reply.send(&encoded);
    }

    pub fn send_ok(self, value: V) {
        self.send(MethodCallResult::Ok(value))
    }

    pub fn send_error(self, code: &str, message: Option<&str>, details: V) {
        self.send(MethodCallResult::Err(MethodCallError {
            code: code.into(),
            message: message.map(|m| m.into()),
            details,
        }));
    }
}

impl<V> Drop for EngineMethodChannel<V> {
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
