use log::warn;

use crate::{
    shell::{Context, EngineHandle},
    Error, Result,
};

use super::{MessageCodec, MethodCall, MethodCallResult, MethodCodec};

// Cloneable invoker that can call channel methods
#[derive(Clone)]
pub struct MethodInvoker<V>
where
    V: 'static,
{
    context: Context,
    engine_handle: EngineHandle,
    channel_name: String,
    codec: &'static dyn MethodCodec<V>,
}

impl<V> MethodInvoker<V> {
    pub fn new(
        context: Context,
        engine_handle: EngineHandle,
        channel_name: String,
        codec: &'static dyn MethodCodec<V>,
    ) -> Self {
        Self {
            context,
            engine_handle,
            channel_name,
            codec,
        }
    }

    pub fn call_method<F>(&self, method: &str, args: V, reply: F) -> Result<()>
    where
        F: FnOnce(MethodCallResult<V>) + 'static,
    {
        if let Some(context) = self.context.get() {
            let encoded = self.codec.encode_method_call(&MethodCall {
                method: method.into(),
                args,
            });
            let engine_manager = context.engine_manager.borrow();
            let engine = engine_manager.get_engine(self.engine_handle);
            if let Some(engine) = engine {
                let codec = self.codec;
                engine.binary_messenger().send_message(
                    &self.channel_name,
                    &encoded,
                    move |message| {
                        if message.is_empty() {
                            // This can happen during hot restart. For now ignore.
                            warn!("Received empty response from isolate");
                        } else {
                            let message = codec.decode_envelope(message).unwrap();
                            reply(message);
                        }
                    },
                )
            } else {
                Err(Error::InvalidEngineHandle)
            }
        } else {
            Err(Error::InvalidContext)
        }
    }
}

//
//
//

#[derive(Clone)]
pub struct EventSender<V>
where
    V: 'static,
{
    context: Context,
    engine_handle: EngineHandle,
    channel_name: String,
    codec: &'static dyn MethodCodec<V>,
}

impl<V> EventSender<V> {
    pub fn new(
        context: Context,
        engine_handle: EngineHandle,
        channel_name: String,
        codec: &'static dyn MethodCodec<V>,
    ) -> Self {
        Self {
            context,
            engine_handle,
            channel_name,
            codec,
        }
    }

    pub fn send_event(&self, message: &V) -> Result<()> {
        if let Some(context) = self.context.get() {
            let encoded = self.codec.encode_success_envelope(message);
            let engine_manager = context.engine_manager.borrow();
            let engine = engine_manager.get_engine(self.engine_handle);
            if let Some(engine) = engine {
                engine
                    .binary_messenger()
                    .post_message(&self.channel_name, &encoded)
            } else {
                Err(Error::InvalidEngineHandle)
            }
        } else {
            Err(Error::InvalidContext)
        }
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
    context: Context,
    engine_handle: EngineHandle,
    channel_name: String,
    codec: &'static dyn MessageCodec<V>,
}

impl<V> MessageSender<V> {
    pub fn new(
        context: Context,
        engine_handle: EngineHandle,
        channel_name: String,
        codec: &'static dyn MessageCodec<V>,
    ) -> Self {
        Self {
            context,
            engine_handle,
            channel_name,
            codec,
        }
    }

    pub fn send_message<F>(&self, message: &V, reply: F) -> Result<()>
    where
        F: FnOnce(V) + 'static,
    {
        if let Some(context) = self.context.get() {
            let encoded = self.codec.encode_message(message);
            let engine_manager = context.engine_manager.borrow();
            let engine = engine_manager.get_engine(self.engine_handle);
            if let Some(engine) = engine {
                let codec = self.codec;
                engine.binary_messenger().send_message(
                    &self.channel_name,
                    &encoded,
                    move |message| {
                        let message = codec.decode_message(message).unwrap();
                        reply(message);
                    },
                )
            } else {
                Err(Error::InvalidEngineHandle)
            }
        } else {
            Err(Error::InvalidContext)
        }
    }

    pub fn post_message(&self, message: &V) -> Result<()> {
        if let Some(context) = self.context.get() {
            let encoded = self.codec.encode_message(message);
            let engine_manager = context.engine_manager.borrow();
            let engine = engine_manager.get_engine(self.engine_handle);
            if let Some(engine) = engine {
                engine
                    .binary_messenger()
                    .post_message(&self.channel_name, &encoded)
            } else {
                Err(Error::InvalidEngineHandle)
            }
        } else {
            Err(Error::InvalidContext)
        }
    }
}
