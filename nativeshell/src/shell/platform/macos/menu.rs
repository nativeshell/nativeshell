use super::{
    error::PlatformResult,
    utils::{superclass, to_nsstring},
};
use crate::{
    shell::{
        api_model::{Accelerator, CheckStatus, Menu, MenuItem, MenuItemRole, MenuRole},
        Context, Handle, MenuDelegate, MenuHandle, MenuManager,
    },
    util::{update_diff, DiffResult, LateRefCell},
};
use cocoa::{
    appkit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem},
    base::{id, nil, NO, YES},
    foundation::{NSInteger, NSUInteger},
};
use lazy_static::lazy_static;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    fmt::Write,
    hash::Hash,
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};

struct StrongPtrWrapper(StrongPtr);

impl PartialEq for StrongPtrWrapper {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for StrongPtrWrapper {}

impl Hash for StrongPtrWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

pub struct PlatformMenuManager {
    context: Context,
    weak_self: LateRefCell<Weak<PlatformMenuManager>>,
    app_menu: RefCell<Option<Rc<PlatformMenu>>>,
    window_menus: RefCell<HashMap<StrongPtrWrapper, Rc<PlatformMenu>>>,
    update_handle: RefCell<Option<Handle>>,
}

impl PlatformMenuManager {
    pub fn new(context: Context) -> Self {
        Self {
            context,
            weak_self: LateRefCell::new(),
            app_menu: RefCell::new(None),
            window_menus: RefCell::new(HashMap::new()),
            update_handle: RefCell::new(None),
        }
    }

    pub(crate) fn assign_weak_self(&self, weak_self: Weak<PlatformMenuManager>) {
        self.weak_self.set(weak_self);
    }

    fn update_menu(&self) {
        unsafe {
            let mut menu = self.app_menu.borrow().clone();
            let app = NSApplication::sharedApplication(nil);
            let key: id = msg_send![app, keyWindow];
            if key != nil {
                let key = StrongPtr::retain(key);
                menu = self
                    .window_menus
                    .borrow()
                    .get(&StrongPtrWrapper(key))
                    .cloned()
                    .or(menu);
            }
            match menu {
                Some(menu) => {
                    let current: id = msg_send![app, mainMenu];
                    if current != *menu.menu {
                        menu.set_as_app_menu();
                    }
                }
                None => {
                    let () = msg_send![app, setMainMenu: nil];
                }
            }
        }
    }

    fn schedule_update(&self) {
        let weak_self = self.weak_self.borrow().clone();
        if let Some(context) = self.context.get() {
            let callback = context.run_loop.borrow().schedule_now(move || {
                if let Some(s) = weak_self.upgrade() {
                    s.update_menu();
                }
            });
            self.update_handle.borrow_mut().replace(callback);
        }
    }

    pub fn set_app_menu(&self, menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        match menu {
            Some(menu) => {
                self.app_menu.borrow_mut().replace(menu);
            }
            None => {
                self.app_menu.borrow_mut().take();
            }
        }
        self.schedule_update();
        Ok(())
    }

    pub fn set_menu_for_window(&self, window: StrongPtr, menu: Option<Rc<PlatformMenu>>) {
        match menu {
            Some(menu) => {
                self.window_menus
                    .borrow_mut()
                    .insert(StrongPtrWrapper(window), menu);
            }
            None => {
                self.window_menus
                    .borrow_mut()
                    .remove(&StrongPtrWrapper(window));
            }
        }
        self.schedule_update();
    }

    pub fn window_will_close(&self, window: StrongPtr) {
        self.window_menus
            .borrow_mut()
            .remove(&StrongPtrWrapper(window));
        self.schedule_update();
    }

    pub fn window_did_become_active(&self, _window: StrongPtr) {
        self.schedule_update();
    }
}

pub struct PlatformMenu {
    handle: MenuHandle,
    pub(super) menu: StrongPtr,
    previous_menu: RefCell<Menu>,
    id_to_menu_item: RefCell<HashMap<i64, StrongPtr>>,
    target: StrongPtr,
    weak_self: LateRefCell<Weak<PlatformMenu>>,
    delegate: Weak<RefCell<dyn MenuDelegate>>,
}

const ITEM_TAG: NSInteger = 9999;

impl PlatformMenu {
    pub fn new(
        _context: Context,
        handle: MenuHandle,
        delegate: Weak<RefCell<dyn MenuDelegate>>,
    ) -> Self {
        unsafe {
            let menu: id = NSMenu::alloc(nil).initWithTitle_(*to_nsstring(""));
            let () = msg_send![menu, setAutoenablesItems: NO];

            let target: id = msg_send![MENU_ITEM_TARGET_CLASS.0, new];
            let target = StrongPtr::new(target);

            let () = msg_send![menu, setDelegate:*target];

            Self {
                handle,
                menu: StrongPtr::new(menu),
                previous_menu: RefCell::new(Default::default()),
                id_to_menu_item: RefCell::new(HashMap::new()),
                target,
                weak_self: LateRefCell::new(),
                delegate,
            }
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {
        self.weak_self.set(weak.clone());
        unsafe {
            let state_ptr = weak.into_raw() as *mut c_void;
            (**self.target).set_ivar("imState", state_ptr);
        }
    }

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        let mut previous_menu = self.previous_menu.borrow_mut();

        let diff = update_diff(&previous_menu.items, &menu.items, |a, b| {
            Self::can_update(a, b)
        });

        // First remove items from menu; This is necessary in case we're reordering a
        // item with submenu - we have to remove it first otherwise we get exception
        // if adding same submenu while it already exists
        let diff: Vec<_> = diff
            .iter()
            .filter(|res| match res {
                DiffResult::Remove(res) => {
                    let item = self.id_to_menu_item.borrow_mut().remove(&res.id);
                    if let Some(item) = item {
                        unsafe {
                            // remove submenu, just in case
                            let () = msg_send![*item, setMenu: nil];
                            let () = msg_send![*self.menu, removeItem:*item];
                        }
                    }
                    false
                }
                _ => true,
            })
            .collect();

        for (i, d) in diff.iter().enumerate() {
            match d {
                DiffResult::Remove(_) => {
                    panic!("Should have been already removed.")
                }
                DiffResult::Keep(_, _) => {
                    // nothing
                }
                DiffResult::Update(old, new) => {
                    let item = self
                        .id_to_menu_item
                        .borrow_mut()
                        .remove(&old.id)
                        .unwrap()
                        .clone();
                    self.id_to_menu_item
                        .borrow_mut()
                        .insert(new.id, item.clone());
                    self.update_menu_item(*item, new, manager);
                }
                DiffResult::Insert(item) => {
                    let menu_item = self.create_menu_item(item, manager);
                    self.id_to_menu_item
                        .borrow_mut()
                        .insert(item.id, menu_item.clone());
                    unsafe { msg_send![*self.menu, insertItem:*menu_item atIndex:i as NSInteger] }
                }
            }
        }

        *previous_menu = menu;

        assert!(
            previous_menu.items.len() == self.id_to_menu_item.borrow().len(),
            "Array length mismatch"
        );

        Ok(())
    }

    fn prepare_for_app_menu(&self) {
        match self.previous_menu.borrow().role {
            Some(MenuRole::Window) => unsafe {
                // Remove all items that don't have our tags; These were added by cocoa; Not doing this
                // will result in duplicate items
                let items: NSUInteger = msg_send![*self.menu, numberOfItems];
                for i in (0..items).rev() {
                    let item: id = msg_send![*self.menu, itemAtIndex: i];
                    let tag: NSInteger = msg_send![item, tag];
                    if tag != ITEM_TAG {
                        let () = msg_send![*self.menu, removeItemAtIndex: i];
                    }
                }

                let app = NSApplication::sharedApplication(nil);
                NSApplication::setWindowsMenu_(app, *self.menu);
                let () = msg_send![*self.menu, setAutoenablesItems: YES];
            },
            Some(MenuRole::Services) => unsafe {
                let app = NSApplication::sharedApplication(nil);
                NSApplication::setServicesMenu_(app, *self.menu);
            },
            None => {}
        };

        let children: Vec<MenuHandle> = self
            .previous_menu
            .borrow()
            .items
            .iter()
            .filter_map(|f| f.submenu)
            .collect();

        if let Some(delegate) = self.delegate.upgrade() {
            for c in children {
                let menu = delegate.borrow().get_platform_menu(c);
                if let Ok(menu) = menu {
                    menu.prepare_for_app_menu();
                }
            }
        }
    }

    fn set_as_app_menu(&self) {
        unsafe {
            let app = NSApplication::sharedApplication(nil);
            let () = msg_send![app, setWindowsMenu: nil];
            self.prepare_for_app_menu();
            let () = msg_send![app, setMainMenu: *self.menu];
        }
    }

    fn can_update(old_item: &MenuItem, new_item: &MenuItem) -> bool {
        // can't change separator item to non separator
        old_item.separator == new_item.separator
    }

    fn update_menu_item(&self, item: id, menu_item: &MenuItem, menu_manager: &MenuManager) {
        if menu_item.separator {
            return;
        }
        unsafe {
            match &menu_item.role {
                Some(role) => {
                    self.update_from_role(item, &menu_item.title, role.clone());
                }
                None => {
                    self.update_from_menu_item(item, menu_item, menu_manager);
                }
            }
        }
    }

    unsafe fn update_from_role(&self, item: id, title: &str, role: MenuItemRole) {
        let () = msg_send![item, setTitle:*to_nsstring(&remove_mnemonics(title))];
        let () = msg_send![item, setTarget: nil];
        match role {
            MenuItemRole::Hide => {
                let () = msg_send![item, setAction: sel!(hide:)];
                let () = msg_send![item, setKeyEquivalent: to_nsstring("h")];
                let () = msg_send![
                    item,
                    setKeyEquivalentModifierMask: NSEventModifierFlags::NSCommandKeyMask
                ];
            }
            MenuItemRole::HideOtherApplications => {
                let () = msg_send![item, setAction: sel!(hideOtherApplications:)];
                let () = msg_send![item, setKeyEquivalent: to_nsstring("h")];
                let () = msg_send![
                    item,
                    setKeyEquivalentModifierMask: NSEventModifierFlags::NSCommandKeyMask
                        | NSEventModifierFlags::NSShiftKeyMask
                ];
            }
            MenuItemRole::ShowAll => {
                let () = msg_send![item, setAction: sel!(unhideAllApplications:)];
            }
            MenuItemRole::QuitApplication => {
                let () = msg_send![item, setAction: sel!(terminate:)];
                let () = msg_send![item, setKeyEquivalent: to_nsstring("q")];
                let () = msg_send![
                    item,
                    setKeyEquivalentModifierMask: NSEventModifierFlags::NSCommandKeyMask
                ];
            }
            MenuItemRole::MinimizeWindow => {
                let () = msg_send![item, setAction: sel!(performMiniaturize:)];
                let () = msg_send![item, setKeyEquivalent: to_nsstring("m")];
                let () = msg_send![
                    item,
                    setKeyEquivalentModifierMask: NSEventModifierFlags::NSCommandKeyMask
                ];
            }
            MenuItemRole::ZoomWindow => {
                let () = msg_send![item, setAction: sel!(performZoom:)];
            }
            MenuItemRole::BringAllToFront => {
                let () = msg_send![item, setAction: sel!(arrangeInFront:)];
            }
        }
    }

    fn accelerator_label_to_string(&self, accelerator: &Accelerator) -> String {
        let label = accelerator.label.to_lowercase();
        let value = match label.as_str() {
            // these must match label descriptions from accelerators.dart
            "f1" => 0xF704,
            "f2" => 0xF705,
            "f3" => 0xF706,
            "f4" => 0xF707,
            "f5" => 0xF708,
            "f6" => 0xF709,
            "f7" => 0xF70A,
            "f8" => 0xF70B,
            "f9" => 0xF70C,
            "f10" => 0xF70D,
            "f11" => 0xF70E,
            "f12" => 0xF70F,
            "home" => 0xF729,
            "end" => 0xF72B,
            "insert" => 0xF727,
            "delete" => 0xF728,
            "backspace" => 0x0008,
            "page up" => 0xF72C,
            "page down" => 0xF72D,
            "space" => 0x0020,
            "tab" => 0x0009,
            "enter" => 0x000d,
            "arrow up" => 0xF700,
            "arrow down" => 0xF701,
            "arrow left" => 0xF702,
            "arrow right" => 0xF703,
            _ => label.chars().next().unwrap_or(0 as char) as u32,
        };
        let mut res = String::new();
        if value > 0 {
            res.push(std::char::from_u32(value).unwrap());
        }
        res
    }

    fn accelerator_label_to_modifier_flags(
        &self,
        accelerator: &Accelerator,
    ) -> NSEventModifierFlags {
        let mut res = NSEventModifierFlags::empty();
        if accelerator.alt {
            res |= NSEventModifierFlags::NSAlternateKeyMask;
        }
        if accelerator.meta {
            res |= NSEventModifierFlags::NSCommandKeyMask;
        }
        if accelerator.control {
            res |= NSEventModifierFlags::NSControlKeyMask;
        }
        if accelerator.shift {
            res |= NSEventModifierFlags::NSShiftKeyMask;
        }

        res
    }

    unsafe fn update_from_menu_item(
        &self,
        item: id,
        menu_item: &MenuItem,
        menu_manager: &MenuManager,
    ) {
        let menu_item_title = to_nsstring(&remove_mnemonics(&menu_item.title));

        if let Some(submenu) = menu_item
            .submenu
            .and_then(|s| menu_manager.get_platform_menu(s).ok())
        {
            let () = msg_send![item, setSubmenu:*submenu.menu];
            let () = msg_send![*submenu.menu, setTitle:*menu_item_title];
            let () = msg_send![item, setTarget: nil];
            let () = msg_send![item, setAction: nil];
        } else {
            let () = msg_send![item, setSubmenu: nil];
            let () = msg_send![item, setTarget: *self.target];
            let () = msg_send![item, setAction: sel!(onAction:)];

            if let Some(accelerator) = &menu_item.accelerator {
                let str = self.accelerator_label_to_string(accelerator);
                if !str.is_empty() {
                    let () = msg_send![item, setKeyEquivalent: to_nsstring(&str)];
                    let () = msg_send![item, setKeyEquivalentModifierMask:
                        self.accelerator_label_to_modifier_flags(accelerator)];
                }
            }
        }

        let () = msg_send![item, setTitle:*menu_item_title];
        let () = msg_send![item, setEnabled:menu_item.enabled];
        let state: NSInteger = {
            match menu_item.check_status == CheckStatus::CheckOn
                || menu_item.check_status == CheckStatus::RadioOn
            {
                true => 1,
                false => 0,
            }
        };
        let () = msg_send![item, setState: state];
        let number: id = msg_send![class!(NSNumber), numberWithLongLong:menu_item.id];
        let () = msg_send![item, setRepresentedObject: number];
    }

    fn menu_item_action(&self, item: id) {
        if let Some(delegate) = self.delegate.upgrade() {
            let item_id = unsafe {
                let object: id = msg_send![item, representedObject];
                msg_send![object, longLongValue]
            };
            delegate.borrow().on_menu_action(self.handle, item_id);
        }
    }

    fn on_menu_will_open(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.borrow().on_menu_open(self.handle);
        }
    }

    fn create_menu_item(&self, menu_item: &MenuItem, menu_manager: &MenuManager) -> StrongPtr {
        unsafe {
            if menu_item.separator {
                let res = NSMenuItem::separatorItem(nil);
                StrongPtr::retain(res)
            } else {
                let res = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                    *to_nsstring(""),
                    Sel::from_ptr(std::ptr::null()),
                    *to_nsstring(""),
                );
                let () = msg_send![res, setTag: ITEM_TAG];
                self.update_menu_item(res, menu_item, menu_manager);
                StrongPtr::new(res)
            }
        }
    }
}

struct MenuItemTargetClass(*const Class);
// Send is required when other dependencies apply the lazy_static feature 'spin_no_std'
unsafe impl Send for MenuItemTargetClass {}
unsafe impl Sync for MenuItemTargetClass {}

lazy_static! {
    static ref MENU_ITEM_TARGET_CLASS: MenuItemTargetClass = unsafe {
        let target_superclass = class!(NSObject);
        let mut decl = ClassDecl::new("IMMenuItemTarget", target_superclass).unwrap();

        decl.add_ivar::<*mut c_void>("imState");

        decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
        decl.add_method(
            sel!(onAction:),
            on_action as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(menuWillOpen:),
            menu_will_open as extern "C" fn(&Object, Sel, id),
        );

        MenuItemTargetClass(decl.register())
    };
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const PlatformMenu
        };
        Weak::from_raw(state_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn on_action(this: &Object, _sel: Sel, sender: id) {
    let state = unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const PlatformMenu
        };
        ManuallyDrop::new(Weak::from_raw(state_ptr))
    };
    let upgraded = state.upgrade();
    if let Some(upgraded) = upgraded {
        upgraded.menu_item_action(sender);
    }
}

extern "C" fn menu_will_open(this: &Object, _: Sel, _menu: id) {
    let state = unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("imState");
            state_ptr as *const PlatformMenu
        };
        ManuallyDrop::new(Weak::from_raw(state_ptr))
    };
    let upgraded = state.upgrade();
    if let Some(upgraded) = upgraded {
        upgraded.on_menu_will_open();
    }
}

#[allow(clippy::branches_sharing_code)]
fn remove_mnemonics(title: &str) -> String {
    let mut res = String::new();
    let mut mnemonic = false;
    for c in title.chars() {
        if c == '&' {
            if !mnemonic {
                mnemonic = true;
                continue;
            } else {
                res.write_char('&').unwrap();
                mnemonic = false;
                continue;
            }
        }
        res.write_char(c).unwrap();
    }
    res
}
