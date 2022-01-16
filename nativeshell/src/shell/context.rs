use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use futures::Future;

use crate::{util::LateRefCell, Error, Result};

use super::{
    platform::{
        app_delegate::ApplicationDelegateManager, drag_data::DragDataAdapter,
        engine::PlatformPlugin, init::init_platform,
    },
    screen_manager::ScreenManager,
    status_item_manager::StatusItemManager,
    EngineManager, HotKeyManager, JoinHandle, KeyboardMapManager, MenuManager, MessageManager,
    RegisteredMethodCallHandler, RunLoop, WindowManager, WindowMethodChannel,
};

pub struct ContextOptions {
    pub app_namespace: String,
    pub flutter_plugins: Vec<PlatformPlugin>,
    pub on_last_engine_removed: Box<dyn Fn(&ContextRef)>,
    pub custom_drag_data_adapters: Vec<Box<dyn DragDataAdapter>>,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            app_namespace: Default::default(),
            flutter_plugins: Vec::new(),
            on_last_engine_removed: Box::new(|context| context.run_loop.borrow().stop()),
            custom_drag_data_adapters: Vec::new(),
        }
    }
}

pub struct ContextImpl {
    pub options: ContextOptions,
    pub run_loop: LateRefCell<RunLoop>,
    pub engine_manager: LateRefCell<EngineManager>,
    pub message_manager: LateRefCell<MessageManager>,
    pub window_method_channel: LateRefCell<WindowMethodChannel>,
    pub window_manager: LateRefCell<WindowManager>,
    pub application_delegate_manager: LateRefCell<ApplicationDelegateManager>,
    pub(crate) menu_manager: LateRefCell<RegisteredMethodCallHandler<MenuManager>>,
    pub(crate) keyboard_map_manager: LateRefCell<RegisteredMethodCallHandler<KeyboardMapManager>>,
    pub(crate) hot_key_manager: LateRefCell<RegisteredMethodCallHandler<HotKeyManager>>,
    pub(crate) screen_manager: LateRefCell<RegisteredMethodCallHandler<ScreenManager>>,
    pub(crate) status_item_manager: LateRefCell<RegisteredMethodCallHandler<StatusItemManager>>,
}

impl ContextImpl {
    fn create(options: ContextOptions) -> Result<ContextRef> {
        let res = Rc::new(Self {
            options,
            run_loop: LateRefCell::new(),
            engine_manager: LateRefCell::new(),
            message_manager: LateRefCell::new(),
            window_method_channel: LateRefCell::new(),
            window_manager: LateRefCell::new(),
            application_delegate_manager: LateRefCell::new(),
            menu_manager: LateRefCell::new(),
            keyboard_map_manager: LateRefCell::new(),
            hot_key_manager: LateRefCell::new(),
            screen_manager: LateRefCell::new(),
            status_item_manager: LateRefCell::new(),
        });
        let res = ContextRef { context: res };
        res.initialize(&res)?;
        Ok(res)
    }

    fn initialize(&self, context: &ContextRef) -> Result<()> {
        init_platform().map_err(Error::from)?;

        self.run_loop.set(RunLoop::new(context));
        self.engine_manager.set(EngineManager::new(context));
        self.message_manager.set(MessageManager::new(context));
        self.window_method_channel
            .set(WindowMethodChannel::new(context));
        self.window_manager.set(WindowManager::new(context));
        self.application_delegate_manager
            .set(ApplicationDelegateManager::new(context));
        self.menu_manager.set(MenuManager::new(context.weak()));
        self.keyboard_map_manager
            .set(KeyboardMapManager::new(context.weak()));
        self.hot_key_manager.set(HotKeyManager::new(context.weak()));
        self.screen_manager.set(ScreenManager::new(context.weak()));
        self.status_item_manager
            .set(StatusItemManager::new(context.weak()));

        #[cfg(debug_assertions)]
        {
            self.sponsor_prompt();
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn sponsor_prompt(&self) {
        if std::env::var("NATIVESHELL_SPONSOR").ok().is_none() {
            println!();
            println!("** Help me make NativeShell and Flutter on desktop better!");
            println!("** We have a long way to go: https://nativeshell.dev/roadmap");
            println!();
        }
    }
}

impl Drop for ContextImpl {
    fn drop(&mut self) {
        self.engine_manager.borrow_mut().shut_down().ok();
    }
}

// Non owning cloneable reference to a Context. Call Context::get() to access the context.
#[derive(Clone)]
pub struct Context {
    context: Weak<ContextImpl>,
}

impl Context {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(options: ContextOptions) -> Result<ContextRef> {
        ContextImpl::create(options)
    }

    pub fn get(&self) -> Option<ContextRef> {
        self.context.upgrade().map(|c| ContextRef { context: c })
    }

    pub fn current() -> Option<ContextRef> {
        CURRENT_CONTEXT.with(|c| c.borrow().as_ref().and_then(|c| c.get()))
    }
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
}

pub struct CurrentContextHandle {
    previous: Option<Context>,
}

impl Drop for CurrentContextHandle {
    fn drop(&mut self) {
        CURRENT_CONTEXT.with(|c| *c.borrow_mut() = self.previous.take());
    }
}

// Strong reference to a Context. Intentionally not clonable; There should be one
// "master" owning reference and all other instances should be used just locally.
// If you need to pass the context along, use weak() to get weak reference and pass that.
pub struct ContextRef {
    context: Rc<ContextImpl>,
}

impl ContextRef {
    pub fn weak(&self) -> Context {
        Context {
            context: Rc::downgrade(&self.context),
        }
    }

    // Sets the context as the current context for the current thread.
    // The context is set as current while the result handle is in scope.
    pub fn set_as_current(&self) -> CurrentContextHandle {
        CurrentContextHandle {
            previous: CURRENT_CONTEXT.with(|c| c.borrow_mut().replace(self.weak())),
        }
    }
}

// Spawn the future with current run loop as executor. This must be called on main
// thread otherwise the method will panic (because there is no context associated
// with current thread).
pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    let context = Context::current().unwrap();
    let run_loop = context.run_loop.borrow();
    run_loop.spawn(future)
}

impl std::ops::Deref for ContextRef {
    type Target = ContextImpl;

    fn deref(&self) -> &ContextImpl {
        self.context.deref()
    }
}
