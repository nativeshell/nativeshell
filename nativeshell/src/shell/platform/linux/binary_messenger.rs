use crate::shell::BinaryMessengerReply;

use super::{
    error::PlatformResult,
    flutter::{self, BinaryMessengerExt},
};

pub struct PlatformBinaryMessenger {
    messenger: flutter::BinaryMessenger,
}

#[allow(unused_variables)]
impl PlatformBinaryMessenger {
    pub fn new(messenger: flutter::BinaryMessenger) -> Self {
        Self { messenger }
    }

    pub fn register_channel_handler<F>(&self, channel: &str, callback: F)
    where
        F: Fn(&[u8], BinaryMessengerReply) -> () + 'static,
    {
        self.messenger.set_message_handler_on_channel(
            channel,
            move |bytes, channel, messenger, response| {
                let reply = BinaryMessengerReply::new(move |data| {
                    messenger.send_response(response, data.into());
                });
                callback(&bytes, reply);
            },
        );
    }

    pub fn unregister_channel_handler(&self, channel: &str) {
        self.messenger.remove_message_handler_on_channel(channel);
    }

    pub fn send_message<F>(&self, channel: &str, message: &[u8], reply: F) -> PlatformResult<()>
    where
        F: FnOnce(&[u8]) -> () + 'static,
    {
        self.messenger
            .send_message(channel, message.into(), move |data| {
                reply(&data);
            });
        Ok(())
    }

    pub fn post_message(&self, channel: &str, message: &[u8]) -> PlatformResult<()> {
        self.messenger.post_message(channel, message.into());
        Ok(())
    }
}
