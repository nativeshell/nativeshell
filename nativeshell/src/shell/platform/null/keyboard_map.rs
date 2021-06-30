use std::{cell::RefCell, rc::Weak};

use crate::shell::{api_model::KeyboardMap, Context, KeyboardMapDelegate};

pub struct PlatformKeyboardMap {}

impl PlatformKeyboardMap {
    pub fn new(context: Context, delegate: Weak<RefCell<dyn KeyboardMapDelegate>>) -> Self {
        Self {}
    }

    pub fn get_current_map(&self) -> KeyboardMap {
        KeyboardMap { keys: vec![] }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformKeyboardMap>) {}
}
