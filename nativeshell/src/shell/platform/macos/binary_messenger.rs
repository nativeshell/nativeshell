use std::{cell::RefCell, collections::HashSet, rc::Rc};

use super::error::PlatformResult;
use crate::shell::BinaryMessengerReply;
use block::{Block, ConcreteBlock, RcBlock};
use cocoa::{
    base::{id, nil},
    foundation::NSString,
};
use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};

pub struct PlatformBinaryMessenger {
    handle: StrongPtr,
    registered: RefCell<HashSet<String>>,
}

impl PlatformBinaryMessenger {
    pub fn from_handle(handle: StrongPtr) -> Self {
        PlatformBinaryMessenger {
            handle,
            registered: RefCell::new(HashSet::new()),
        }
    }

    pub fn register_channel_handler<F>(&self, channel: &str, callback: F)
    where
        F: Fn(&[u8], BinaryMessengerReply) + 'static,
    {
        unsafe {
            let closure = move |data: id, reply: &mut Block<(id,), ()>| {
                let bytes: *const u8 = msg_send![data, bytes];
                let length: usize = msg_send![data, length];
                let data: &[u8] = std::slice::from_raw_parts(bytes, length);

                let reply_block = RcBlock::copy(reply as *mut _);
                let cb = move |reply: &[u8]| {
                    let data: id =
                        msg_send![class!(NSData), dataWithBytes:reply.as_ptr() length:reply.len()];
                    reply_block.call((data,));
                };

                callback(data, BinaryMessengerReply::new(cb));
            };

            let block = ConcreteBlock::new(closure);
            let block = block.copy();

            let channel = StrongPtr::new(NSString::alloc(nil).init_str(channel));

            // macos embedder doesn't return anything useful here; It also leaks the block sadly
            let () = msg_send![*self.handle, setMessageHandlerOnChannel:*channel binaryMessageHandler:&*block];
        }

        self.registered.borrow_mut().insert(channel.into());
    }

    pub fn unregister_channel_handler(&self, channel: &str) {
        self.registered.borrow_mut().remove(channel);
        unsafe {
            let channel = StrongPtr::new(NSString::alloc(nil).init_str(channel));
            let () = msg_send![*self.handle, setMessageHandlerOnChannel:*channel binaryMessageHandler:nil];
        }
    }

    pub fn send_message<F>(&self, channel: &str, message: &[u8], reply: F) -> PlatformResult<()>
    where
        F: FnOnce(&[u8]) + 'static,
    {
        // we know we're going to be called only once, but rust doesn't
        let r = Rc::new(RefCell::new(Some(reply)));
        unsafe {
            let closure = move |data: id| {
                let bytes: *const u8 = msg_send![data, bytes];
                let length: usize = msg_send![data, length];
                let data: &[u8] = std::slice::from_raw_parts(bytes, length);
                let reply = r.clone();
                let function = reply.borrow_mut().take().unwrap();
                function(data);
            };
            let block = ConcreteBlock::new(closure);
            let block = block.copy();
            let channel = StrongPtr::new(NSString::alloc(nil).init_str(channel));
            let data: id =
                msg_send![class!(NSData), dataWithBytes:message.as_ptr() length:message.len()];

            // sendOnChannel: doesn't have return value :-/
            let () =
                msg_send![*self.handle, sendOnChannel:*channel message:data binaryReply:&*block];
        }

        Ok(())
    }

    pub fn post_message(&self, channel: &str, message: &[u8]) -> PlatformResult<()> {
        unsafe {
            let channel = StrongPtr::new(NSString::alloc(nil).init_str(channel));
            let data: id =
                msg_send![class!(NSData), dataWithBytes:message.as_ptr() length:message.len()];

            // sendOnChannel: doesn't have return value :-/
            let () = msg_send![*self.handle, sendOnChannel:*channel message:data];
        }

        Ok(())
    }
}

impl Drop for PlatformBinaryMessenger {
    fn drop(&mut self) {
        let channels = self.registered.borrow().clone();
        for c in channels.iter() {
            self.unregister_channel_handler(c);
        }
    }
}
