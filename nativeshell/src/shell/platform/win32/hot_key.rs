use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{
            MapVirtualKeyW, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT,
            MOD_CONTROL, MOD_SHIFT, MOD_WIN,
        },
        WindowsAndMessaging::MAPVK_VSC_TO_VK,
    },
};

use super::run_loop::PlatformRunLoopHotKeyDelegate;

use std::{cell::RefCell, collections::HashMap, rc::Weak};

use crate::shell::{
    api_model::Accelerator, Context, EngineHandle, HotKeyHandle, HotKeyManagerDelegate,
};

use super::error::PlatformResult;

pub(crate) struct PlatformHotKeyManager {
    context: Context,
    delegate: Weak<RefCell<dyn HotKeyManagerDelegate>>,
    hot_keys: RefCell<HashMap<HotKeyHandle, EngineHandle>>,
}

impl PlatformRunLoopHotKeyDelegate for PlatformHotKeyManager {
    fn on_hot_key(&self, hot_key: i32) {
        let hot_key = HotKeyHandle(hot_key as i64);
        if let Some(engine) = self.hot_keys.borrow().get(&hot_key) {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.borrow().on_hot_key_pressed(hot_key, *engine);
            }
        }
    }
}

impl PlatformHotKeyManager {
    pub fn new(context: Context, delegate: Weak<RefCell<dyn HotKeyManagerDelegate>>) -> Self {
        Self {
            context,
            delegate,
            hot_keys: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformHotKeyManager>) {
        if let Some(context) = self.context.get() {
            context
                .run_loop
                .borrow()
                .platform_run_loop
                .set_hot_key_delegate(weak);
        }
    }

    fn hwnd(&self) -> HWND {
        if let Some(context) = self.context.get() {
            context.run_loop.borrow().platform_run_loop.hwnd()
        } else {
            HWND(0)
        }
    }

    pub fn create_hot_key(
        &self,
        accelerator: Accelerator,
        virtual_key: i64,
        handle: HotKeyHandle,
        engine: EngineHandle,
    ) -> PlatformResult<()> {
        let mut modifiers = HOT_KEY_MODIFIERS::default();
        if accelerator.alt {
            modifiers |= MOD_ALT;
        }
        if accelerator.control {
            modifiers |= MOD_CONTROL;
        }
        if accelerator.shift {
            modifiers |= MOD_SHIFT;
        }
        if accelerator.meta {
            modifiers |= MOD_WIN;
        }
        self.hot_keys.borrow_mut().insert(handle, engine);
        unsafe {
            let vk = MapVirtualKeyW(virtual_key as u32, MAPVK_VSC_TO_VK);
            RegisterHotKey(self.hwnd(), handle.0 as i32, modifiers, vk);
        }
        Ok(())
    }

    pub fn destroy_hot_key(&self, handle: HotKeyHandle) -> PlatformResult<()> {
        unsafe {
            UnregisterHotKey(self.hwnd(), handle.0 as i32);
        }
        Ok(())
    }

    pub fn engine_destroyed(&self, engine: EngineHandle) -> PlatformResult<()> {
        let hot_keys: Vec<HotKeyHandle> = self
            .hot_keys
            .borrow()
            .iter()
            .filter_map(|(key, e)| if e == &engine { Some(*key) } else { None })
            .collect();
        for key in hot_keys {
            self.destroy_hot_key(key)?;
        }
        Ok(())
    }
}
