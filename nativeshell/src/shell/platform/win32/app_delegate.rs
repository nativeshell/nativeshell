use crate::shell::ContextRef;

pub trait ApplicationDelegate {}

pub struct ApplicationDelegateManager {}

impl ApplicationDelegateManager {
    pub fn new(_context: &ContextRef) -> Self {
        Self {}
    }
    pub fn set_delegate<D: ApplicationDelegate + 'static>(&self, _delegate: D) {}
}
