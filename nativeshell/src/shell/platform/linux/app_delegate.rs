use std::{cell::RefCell, rc::Rc};

use crate::shell::ContextRef;

pub trait ApplicationDelegate {}

pub struct ApplicationDelegateManager {}

impl ApplicationDelegateManager {
    pub fn new(_context: &ContextRef) -> Self {
        Self {}
    }
    pub fn set_delegate<D: ApplicationDelegate + 'static>(&self, _delegate: D) {}
    pub fn set_delegate_ref<D: ApplicationDelegate + 'static>(&self, _delegate: Rc<RefCell<D>>) {}
}
