use std::rc::Rc;

use crate::{
    shell::{BinaryMessengerReply, Context, EngineHandle, EngineManager},
    Error, Result,
};

use super::{MethodCall, MethodCallError, MethodCallResult, MethodCodec};

// Low level interface to method channel on single engine
pub struct EngineMethodChannel<V>
where
    V: 'static,
{
    context: Rc<Context>,
    invoker: MethodInvoker<V>,
}

impl<V> EngineMethodChannel<V> {
    pub fn new<F>(
        context: Rc<Context>,
        engine_handle: EngineHandle,
        channel_name: &str,
        codec: &'static dyn MethodCodec<V>,
        callback: F,
    ) -> Self
    where
        F: Fn(MethodCall<V>, MethodCallReply<V>) + 'static,
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
        codec: &'static dyn MethodCodec<V>,
        callback: F,
        engine_manager: &EngineManager,
    ) -> Self
    where
        F: Fn(MethodCall<V>, MethodCallReply<V>) + 'static,
    {
        let res = EngineMethodChannel {
            context: context.clone(),
            invoker: MethodInvoker {
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
                    let message = codec.decode_method_call(data).unwrap();
                    let reply = MethodCallReply { reply, codec };
                    callback(message, reply);
                });
        }
        res
    }

    pub fn invoker(&self) -> &MethodInvoker<V> {
        &self.invoker
    }
}

//
//
//

// Cloneable invoker that can call channel methods
#[derive(Clone)]
pub struct MethodInvoker<V>
where
    V: 'static,
{
    context: Rc<Context>,
    engine_handle: EngineHandle,
    channel_name: String,
    codec: &'static dyn MethodCodec<V>,
}

impl<V> MethodInvoker<V> {
    pub fn call_method<F>(&self, method: String, args: V, reply: F) -> Result<()>
    where
        F: FnOnce(MethodCallResult<V>) + 'static,
    {
        let encoded = self.codec.encode_method_call(&MethodCall { method, args });
        let engine_manager = self.context.engine_manager.borrow();
        let engine = engine_manager.get_engine(self.engine_handle);
        if let Some(engine) = engine {
            let codec = self.codec;
            engine
                .binary_messenger()
                .send_message(&self.channel_name, &encoded, move |message| {
                    let message = codec.decode_envelope(message).unwrap();
                    reply(message);
                })
        } else {
            Err(Error::InvalidEngineHandle)
        }
    }
}

//
//
//

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
        let engine_manager = self.context.engine_manager.borrow();
        let engine = engine_manager.get_engine(self.invoker.engine_handle);
        if let Some(engine) = engine {
            engine
                .binary_messenger()
                .unregister_channel_handler(&self.invoker.channel_name);
        }
    }
}
