use super::platform::binary_messenger::PlatformBinaryMessenger;
use crate::Result;

pub struct BinaryMessengerReply {
    sent: bool,
    callback: Option<Box<dyn FnOnce(&[u8])>>,
}

impl BinaryMessengerReply {
    pub fn new<F>(callback: F) -> Self
    where
        F: FnOnce(&[u8]) + 'static,
    {
        BinaryMessengerReply {
            sent: false,
            callback: Some(Box::new(callback)),
        }
    }

    pub fn send(mut self, message: &[u8]) {
        self.sent = true;
        let callback = self.callback.take().unwrap();
        callback(message);
    }
}

pub struct BinaryMessenger {
    messenger: PlatformBinaryMessenger,
}

impl BinaryMessenger {
    pub fn new(messenger_impl: PlatformBinaryMessenger) -> Self {
        BinaryMessenger {
            messenger: messenger_impl,
        }
    }

    pub fn register_channel_handler<F>(&self, channel: &str, callback: F)
    where
        F: Fn(&[u8], BinaryMessengerReply) + 'static,
    {
        self.messenger.register_channel_handler(channel, callback);
    }

    pub fn unregister_channel_handler(&self, channel: &str) {
        self.messenger.unregister_channel_handler(channel);
    }

    pub fn send_message<F>(&self, channel: &str, message: &[u8], reply_callback: F) -> Result<()>
    where
        F: FnOnce(&[u8]) + 'static,
    {
        self.messenger
            .send_message(channel, message, reply_callback)
            .map_err(|e| e.into())
    }

    // like "send_message" but wihtout reply
    pub fn post_message(&self, channel: &str, message: &[u8]) -> Result<()> {
        self.messenger
            .post_message(channel, message)
            .map_err(|e| e.into())
    }
}

impl Drop for BinaryMessengerReply {
    fn drop(&mut self) {
        if !self.sent {
            // Send empty reply so that the message doesn't leak
            let callback = self.callback.take().unwrap();
            callback(&[]);
        }
    }
}
