use crate::shell::BinaryMessengerReply;

use super::error::{PlatformError, PlatformResult};

pub struct PlatformBinaryMessenger {}

#[allow(unused_variables)]
impl PlatformBinaryMessenger {
    pub fn register_channel_handler<F>(&self, channel: &str, callback: F)
    where
        F: Fn(&[u8], BinaryMessengerReply) -> () + 'static,
    {
    }

    pub fn unregister_channel_handler(&self, channel: &str) {}

    pub fn send_message<F>(&self, channel: &str, message: &[u8], reply: F) -> PlatformResult<()>
    where
        F: FnOnce(&[u8]) -> () + 'static,
    {
        Err(PlatformError::NotImplemented)
    }

    pub fn post_message(&self, channel: &str, message: &[u8]) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
