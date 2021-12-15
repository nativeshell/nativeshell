use super::utils::from_nsstring;
use crate::shell::{platform::platform_impl::utils::superclass, Context, ContextRef};
use block::{Block, RcBlock};
use cocoa::{
    appkit::{NSApplication, NSApplicationTerminateReply},
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSArray, NSUInteger},
};
use core::panic;
use libc::c_void;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;
use std::{
    cell::{Cell, RefCell},
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};
use url::Url;

pub struct AppTermination {}

impl AppTermination {
    pub fn terminate_reply(should_terminate: bool) {
        unsafe {
            let app = NSApplication::sharedApplication(nil);
            let () = msg_send![app, replyToApplicationShouldTerminate: if should_terminate { YES } else { NO }];
        }
    }
}

pub enum ApplicationTerminateReply {
    Cancel,
    Now,
    Later,
}

pub trait ApplicationDelegate {
    // done
    fn application_will_finish_launching(&mut self) {}
    fn application_did_finish_launching(&mut self) {}

    fn application_will_become_active(&mut self) {}
    fn application_did_become_active(&mut self) {}
    fn application_will_resign_active(&mut self) {}
    fn application_did_resign_active(&mut self) {}

    fn application_should_terminate(
        &mut self,
        _termination: AppTermination,
    ) -> ApplicationTerminateReply {
        ApplicationTerminateReply::Now
    }

    fn application_should_terminate_after_last_window_closed(&mut self) -> bool {
        true
    }

    fn application_will_terminate(&mut self) {}
    fn application_will_hide(&mut self) {}
    fn application_did_hide(&mut self) {}
    fn application_will_unhide(&mut self) {}
    fn application_did_unhide(&mut self) {}

    fn application_will_update(&mut self) {}
    fn application_did_update(&mut self) {}

    fn application_should_handle_reopen(&mut self, _has_visible_windows: bool) -> bool {
        true
    }

    fn application_will_present_error(&mut self, error: id) -> id {
        error
    }

    fn application_did_change_screen_paramaters(&mut self) {}
    fn application_did_change_occlusion_state(&mut self) {}

    fn application_open_urls(&mut self, _urls: &[Url]) {}

    fn application_continue_user_activity(
        &mut self,
        _user_activity: id, /* NSUserActivity */
        _restoration_handler: Box<dyn FnOnce(id /* NSArray<id<NSUserActivityRestoring>> */)>,
    ) -> bool {
        false
    }
}

struct DelegateState {
    context: Context,
    delegate: RefCell<Option<Rc<RefCell<dyn ApplicationDelegate>>>>,
    in_handler: Cell<bool>,
    execute_after: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl DelegateState {
    pub fn execute_after_handler<F: FnOnce() + 'static>(&self, f: F) {
        if !self.in_handler.get() {
            panic!("execute_after_handler must be called during handler invocation.");
        }
        if self.execute_after.borrow().is_some() {
            panic!("execute_after_handler has already been called for this handler invocation.");
        }
        self.execute_after.borrow_mut().replace(Box::new(f));
    }
}

pub struct ApplicationDelegateManager {
    state: Rc<DelegateState>,
    _object: StrongPtr,
}

impl ApplicationDelegateManager {
    pub fn new(context: &ContextRef) -> Self {
        let state = Rc::new(DelegateState {
            context: context.weak(),
            delegate: RefCell::new(None),
            in_handler: Cell::new(false),
            execute_after: RefCell::new(None),
        });
        let object = autoreleasepool(|| unsafe {
            let object: id = msg_send![*APPLICATION_DELEGATE_CLASS, new];
            let weak = Rc::downgrade(&state);
            let state_ptr = weak.into_raw() as *mut c_void;
            (*object).set_ivar("imState", state_ptr);
            let app = NSApplication::sharedApplication(nil);
            let () = msg_send![app, setDelegate: object];
            StrongPtr::new(object)
        });
        Self {
            state,
            _object: object,
        }
    }

    pub fn set_delegate<D: ApplicationDelegate + 'static>(&self, delegate: D) {
        self.state
            .delegate
            .borrow_mut()
            .replace(Rc::new(RefCell::new(delegate)));
    }

    pub fn set_delegate_ref<D: ApplicationDelegate + 'static>(&self, delegate: Rc<RefCell<D>>) {
        self.state.delegate.borrow_mut().replace(delegate);
    }

    // Executes the callback right after current handler returns, in same
    // run loop turn as current handler, but without the handler borrowed.
    // This is useful for situations where the callback might run nested
    // run-loop.
    pub fn execute_after_handler<F: FnOnce() + 'static>(&self, f: F) {
        self.state.execute_after_handler(f);
    }
}

static APPLICATION_DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("IMApplicationDeleagte", superclass).unwrap();
    decl.add_ivar::<*mut c_void>("imState");
    if let Some(protocol) = Protocol::get("NSApplicationDelegate") {
        decl.add_protocol(protocol);
    }
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(applicationWillFinishLaunching:),
        will_finish_launching as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidFinishLaunching:),
        did_finish_launching as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationWillBecomeActive:),
        will_become_active as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidBecomeActive:),
        did_become_active as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationWillResignActive:),
        will_resign_active as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidResignActive:),
        did_resign_active as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationShouldTerminate:),
        should_terminate as extern "C" fn(&Object, Sel, id) -> NSUInteger,
    );
    decl.add_method(
        sel!(applicationShouldTerminateAfterLastWindowClosed:),
        should_terminate_after_last_window_closed as extern "C" fn(&Object, Sel, id) -> BOOL,
    );
    decl.add_method(
        sel!(applicationWillTerminate:),
        will_terminate as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationWillHide:),
        will_hide as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidHide:),
        did_hide as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationWillUnhide:),
        will_unhide as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidUnhide:),
        did_unhide as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationWillUpdate:),
        will_update as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidUpdate:),
        did_update as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationShouldHandleReopen:hasVisibleWindows:),
        should_handle_reopen as extern "C" fn(&Object, Sel, id, BOOL) -> BOOL,
    );
    decl.add_method(
        sel!(application:willPresentError:),
        will_present_error as extern "C" fn(&Object, Sel, id, id) -> id,
    );
    decl.add_method(
        sel!(applicationDidChangeScreenParameters:),
        did_change_screen_parameters as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(applicationDidChangeOcclusionState:),
        did_change_occlusion_state as extern "C" fn(&Object, Sel, id),
    );
    decl.add_method(
        sel!(application:openFiles:),
        open_files as extern "C" fn(&Object, Sel, id, id),
    );
    decl.add_method(
        sel!(application:openURLs:),
        open_urls as extern "C" fn(&Object, Sel, id, id),
    );
    decl.add_method(
        sel!(application:continueUserActivity:restorationHandler:),
        continue_user_activity as extern "C" fn(&Object, Sel, id, id, id),
    );
    decl.register()
});

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const DelegateState
        };
        Weak::from_raw(state_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn will_finish_launching(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_finish_launching();
    });
}

extern "C" fn did_finish_launching(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_finish_launching();
    });
}

extern "C" fn will_become_active(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_become_active();
    });
}

extern "C" fn did_become_active(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_become_active();
    });
}

extern "C" fn will_resign_active(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_resign_active();
    });
}

extern "C" fn did_resign_active(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_resign_active();
    });
}

extern "C" fn should_terminate(this: &Object, _sel: Sel, _sender: id) -> NSUInteger {
    let res = with_delegate_res(
        this,
        |delegate| delegate.application_should_terminate(AppTermination {}),
        || ApplicationTerminateReply::Now,
    );
    match res {
        ApplicationTerminateReply::Cancel => {
            NSApplicationTerminateReply::NSTerminateCancel as NSUInteger
        }
        ApplicationTerminateReply::Now => NSApplicationTerminateReply::NSTerminateNow as NSUInteger,
        ApplicationTerminateReply::Later => {
            NSApplicationTerminateReply::NSTerminateLater as NSUInteger
        }
    }
}

extern "C" fn should_terminate_after_last_window_closed(
    this: &Object,
    _sel: Sel,
    _sender: id,
) -> BOOL {
    let res = with_delegate_res(
        this,
        |delegate| delegate.application_should_terminate_after_last_window_closed(),
        || false,
    );
    match res {
        true => YES,
        false => NO,
    }
}

extern "C" fn will_terminate(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_terminate();
    });
    with_state(this, |state| {
        if let Some(context) = state.context.get() {
            context.engine_manager.borrow_mut().shut_down().ok();
        }
    });
}

extern "C" fn will_hide(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_hide();
    });
}

extern "C" fn did_hide(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_hide();
    });
}

extern "C" fn will_unhide(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_unhide();
    });
}

extern "C" fn did_unhide(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_unhide();
    });
}

extern "C" fn will_update(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_will_update();
    });
}

extern "C" fn did_update(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_update();
    });
}

extern "C" fn should_handle_reopen(
    this: &Object,
    _sel: Sel,
    _sender: id,
    has_visible_windows: BOOL,
) -> BOOL {
    let res = with_delegate_res(
        this,
        |delegate| delegate.application_should_handle_reopen(has_visible_windows != NO),
        || false,
    );
    match res {
        true => YES,
        false => NO,
    }
}

extern "C" fn will_present_error(this: &Object, _sel: Sel, _sender: id, error: id) -> id {
    with_delegate_res(
        this,
        |delegate| delegate.application_will_present_error(error),
        || error,
    )
}

extern "C" fn did_change_screen_parameters(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_change_screen_paramaters();
    });
}

extern "C" fn did_change_occlusion_state(this: &Object, _sel: Sel, _notification: id) {
    with_delegate(this, |delegate| {
        delegate.application_did_change_occlusion_state();
    });
}

extern "C" fn open_files(this: &Object, _sel: Sel, _sender: id, files: id) {
    let mut urls = Vec::<Url>::new();
    unsafe {
        for i in 0..NSArray::count(files) {
            let string = from_nsstring(NSArray::objectAtIndex(files, i));
            if let Ok(url) = Url::parse(&string) {
                urls.push(url);
            }
        }
    }
    with_delegate(this, |delegate| {
        delegate.application_open_urls(&urls);
    })
}

extern "C" fn open_urls(this: &Object, _sel: Sel, _sender: id, urls: id) {
    let mut u = Vec::<Url>::new();
    unsafe {
        for i in 0..NSArray::count(urls) {
            let url = NSArray::objectAtIndex(urls, i);
            let string: id = msg_send![url, absoluteString];
            let string = from_nsstring(string);

            if let Ok(url) = Url::parse(&string) {
                u.push(url);
            }
        }
    }
    with_delegate(this, |delegate| {
        delegate.application_open_urls(&u);
    })
}

extern "C" fn continue_user_activity(
    this: &Object,
    _sel: Sel,
    _sender: id,
    activity: id,
    restoration_handler: id,
) {
    let restoration_handler = restoration_handler as *mut Block<(id,), ()>;
    let restoration_handler = unsafe { RcBlock::copy(restoration_handler) };
    with_delegate(this, |delegate| {
        let f = Box::new(move |a: id| {
            unsafe { restoration_handler.call((a,)) };
        });
        delegate.application_continue_user_activity(activity, f);
    });
}

//

fn with_state<F>(this: &Object, callback: F)
where
    F: FnOnce(Rc<DelegateState>),
{
    let state = unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const DelegateState
        };
        ManuallyDrop::new(Weak::from_raw(state_ptr))
    };
    let upgraded = state.upgrade();
    if let Some(upgraded) = upgraded {
        callback(upgraded);
    }
}

fn with_delegate<F>(this: &Object, callback: F)
where
    F: FnOnce(&mut dyn ApplicationDelegate),
{
    with_state(this, |state| {
        state.in_handler.set(true);
        if let Some(delegate) = state.delegate.borrow().as_ref() {
            let delegate = &mut *delegate.borrow_mut();
            callback(delegate);
        }
        state.in_handler.set(false);
        let cb = state.execute_after.borrow_mut().take();
        if let Some(cb) = cb {
            cb();
        }
    });
}

fn with_delegate_res<F, FR, R>(this: &Object, callback: F, default: FR) -> R
where
    F: FnOnce(&mut dyn ApplicationDelegate) -> R,
    FR: FnOnce() -> R,
{
    let mut res = None::<R>;
    with_state(this, |state| {
        state.in_handler.set(true);
        if let Some(delegate) = state.delegate.borrow().as_ref() {
            let delegate = &mut *delegate.borrow_mut();
            res.replace(callback(delegate));
        }
        state.in_handler.set(false);
        if let Some(cb) = state.execute_after.borrow_mut().take() {
            cb();
        }
    });
    res.unwrap_or_else(default)
}
