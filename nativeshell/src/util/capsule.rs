use std::thread::{self, ThreadId};

use crate::shell::RunLoopSender;

// Thread bound capsule; Allows retrieving the value only on the thread
// where it was stored.
pub struct Capsule<T>
where
    T: 'static,
{
    value: Option<T>,
    thread_id: ThreadId,
    sender: Option<RunLoopSender>,
}

impl<T> Capsule<T>
where
    T: 'static,
{
    // Creates new capsule; If the value is not taken out of capsule, the
    // capsule must be dropped on same thread as it was created, otherwise
    // it will panic
    pub fn new(value: T) -> Self {
        Self {
            value: Some(value),
            thread_id: thread::current().id(),
            sender: None,
        }
    }

    // Creates new capsule, If the value is not taken out of capsule and the
    // capsule is dropped on different thread than where it was created, it will
    // be sent to the sender and dropped on the run loop thread
    pub fn new_with_sender(value: T, sender: RunLoopSender) -> Self {
        Self {
            value: Some(value),
            thread_id: thread::current().id(),
            sender: Some(sender),
        }
    }

    pub fn get_ref(&self) -> Option<&T> {
        if self.thread_id == thread::current().id() {
            self.value.as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.thread_id == thread::current().id() {
            self.value.as_mut()
        } else {
            None
        }
    }

    pub fn take(&mut self) -> Option<T> {
        if self.thread_id == thread::current().id() {
            self.value.take()
        } else {
            None
        }
    }
}

impl<T> Drop for Capsule<T> {
    fn drop(&mut self) {
        // we still have value and capsule was dropped in other thread
        if self.value.is_some() && self.thread_id != thread::current().id() {
            if let Some(sender) = self.sender.as_ref() {
                let carry = Carry(self.value.take().unwrap());
                let thread_id = self.thread_id;
                sender.send(move || {
                    // make sure that sender sent us back to initial thread
                    if thread_id != thread::current().id() {
                        panic!("Capsule was created on different thread than sender target")
                    }
                    let _ = carry;
                });
            } else if !thread::panicking() {
                panic!("Capsule was dropped on wrong thread with data still in it!");
            }
        }
    }
}

unsafe impl<T> Send for Capsule<T> {}

struct Carry<T>(T);

unsafe impl<T> Send for Carry<T> {}
