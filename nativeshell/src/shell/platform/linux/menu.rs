use std::{
    cell::{Cell, RefCell},
    cmp::max,
    collections::HashMap,
    fmt::Write,
    ptr,
    rc::{Rc, Weak},
};

use gdk::ModifierType;
use glib::{Cast, ObjectExt};
use gtk::{
    prelude::{
        AccelLabelExt, BinExt, ContainerExt, GtkMenuExt, GtkMenuItemExt, MenuShellExt, WidgetExt,
    },
    AccelLabel, MenuDirectionType,
};

use crate::{
    shell::{
        api_model::{Accelerator, CheckStatus, Menu, MenuItem},
        Context, MenuDelegate, MenuHandle, MenuManager,
    },
    util::{update_diff, DiffResult, LateRefCell},
};

use super::{
    error::{PlatformError, PlatformResult},
    menu_item::{
        check_menu_item_set_checked, create_check_menu_item, create_radio_menu_item,
        radio_menu_item_set_checked,
    },
};

pub struct PlatformMenu {
    context: Context,
    handle: MenuHandle,
    weak_self: LateRefCell<Weak<PlatformMenu>>,
    pub(super) menu: gtk::Menu,
    previous_menu: RefCell<Menu>,
    id_to_menu_item: RefCell<HashMap<i64, gtk::MenuItem>>,
    item_selected: Cell<bool>,
    on_selection_done: RefCell<Option<Box<dyn FnOnce(bool)>>>,
    ignore_activate: Cell<bool>,
    pending_selection_done: Cell<bool>,
    delegate: Weak<RefCell<dyn MenuDelegate>>,
}

#[allow(unused_variables)]
impl PlatformMenu {
    pub fn new(
        context: Context,
        handle: MenuHandle,
        delegate: Weak<RefCell<dyn MenuDelegate>>,
    ) -> Self {
        let m = gtk::Menu::new();

        Self {
            context,
            handle,
            weak_self: LateRefCell::new(),
            menu: gtk::Menu::new(),
            previous_menu: RefCell::new(Default::default()),
            id_to_menu_item: RefCell::new(HashMap::new()),
            item_selected: Cell::new(false),
            on_selection_done: RefCell::new(None),
            ignore_activate: Cell::new(false),
            pending_selection_done: Cell::new(false),
            delegate,
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {
        self.weak_self.set(weak.clone());
        unsafe {
            self.menu.set_data("nativeshell_platform_menu", weak);
        }

        // find top level menu and fire callback
        self.menu.connect_selection_done(|menu| {
            let menu = Self::top_level_menu(menu);
            if let Some(platform_menu) = Self::platform_menu_from_gtk_menu(&menu) {
                platform_menu.trigger_selection_done();
            }
        });

        let weak = self.weak_self.borrow().clone();
        self.menu.connect_move_current(move |_, dir| {
            if let Some(s) = weak.upgrade() {
                s.on_move_current(dir);
            }
        });

        let weak = self.weak_self.borrow().clone();
        self.menu.connect_show(move |_| {
            if let Some(s) = weak.upgrade() {
                if let Some(delegate) = s.delegate.upgrade() {
                    delegate.borrow().on_menu_open(s.handle);
                }
            }
        });

        self.menu.connect_hide(move |menu| {
            if let Some(platform_menu) = Self::platform_menu_from_gtk_menu(menu) {
                platform_menu.set_pending_selection_done();
                // Fix for https://github.com/nativeshell/examples/issues/13
                // Sometimes on KDE/Wayland when activating another window the
                // selection_done event is not fired. So we trigger it here.
                let platform_menu_clone = platform_menu.clone();
                if let Some(context) = platform_menu.context.get() {
                    context
                        .run_loop
                        .borrow()
                        .schedule_now(move || {
                            platform_menu_clone.trigger_selection_done();
                        })
                        .detach();
                }
            }
        });
    }

    // Callback will be fired if item in this menu or any submenu is selected
    pub fn on_selection_done<F: FnOnce(/*item_selected:*/ bool) + 'static>(&self, callback: F) {
        self.item_selected.replace(false);
        self.on_selection_done
            .borrow_mut()
            .replace(Box::new(callback));
    }

    pub fn set_pending_selection_done(&self) {
        self.pending_selection_done.set(true);
    }

    pub fn trigger_selection_done(&self) {
        if self.pending_selection_done.replace(false) {
            let done = self.on_selection_done.borrow_mut().take();
            if let Some(done) = done {
                done(self.item_selected.get());
            }
        }
    }

    fn platform_menu_from_gtk_menu(menu: &gtk::Menu) -> Option<Rc<PlatformMenu>> {
        let platform_menu: Option<ptr::NonNull<Weak<PlatformMenu>>> =
            unsafe { menu.data("nativeshell_platform_menu") };
        platform_menu.and_then(|m| unsafe { m.as_ref() }.upgrade())
    }

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        let mut previous_menu = self.previous_menu.borrow_mut();

        let diff = update_diff(&previous_menu.items, &menu.items, |a, b| {
            Self::can_update(a, b)
        });

        // First remove items from menu
        let diff: Vec<_> = diff
            .iter()
            .filter(|res| match res {
                DiffResult::Remove(res) => {
                    let item = self.id_to_menu_item.borrow_mut().remove(&res.id);
                    if let Some(item) = item {
                        self.menu.remove(&item);
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
                    self.update_menu_item(&item, new, manager);
                }
                DiffResult::Insert(item) => {
                    let menu_item = self.create_menu_item(item, manager);
                    self.id_to_menu_item
                        .borrow_mut()
                        .insert(item.id, menu_item.clone());
                    self.menu.insert(&menu_item, i as i32);
                }
            }
        }

        *previous_menu = menu;

        assert!(
            previous_menu.items.len() == self.id_to_menu_item.borrow().len(),
            "Array length mismatch"
        );

        // For whatever reason Gtk doesn't bother resizing menus when items change
        // (i.e. label gets longer), which results in "attempt to underallocate" log spam
        // and menu cut off
        self.resize_menu_if_needed();

        Ok(())
    }

    fn resize_menu_if_needed(&self) {
        let top_level = self.menu.toplevel();
        let win = top_level.as_ref().and_then(|w| w.window());
        if let (Some(win), Some(top_level)) = (win, top_level) {
            if win.is_visible() {
                let natural_size = top_level.preferred_size().1;
                let width = win.width();
                let height = win.height();

                if width < natural_size.width || height < natural_size.height {
                    win.resize(
                        max(width, natural_size.width),
                        max(height, natural_size.height),
                    );
                }
            }
        }
    }

    fn create_menu_item(&self, menu_item: &MenuItem, menu_manager: &MenuManager) -> gtk::MenuItem {
        let res = if menu_item.separator {
            gtk::SeparatorMenuItem::new().upcast::<gtk::MenuItem>()
        } else {
            let res = if menu_item.check_status == CheckStatus::None {
                gtk::MenuItem::new()
            } else if menu_item.check_status == CheckStatus::CheckOn
                || menu_item.check_status == CheckStatus::CheckOff
            {
                create_check_menu_item().upcast::<gtk::MenuItem>()
            } else if menu_item.check_status == CheckStatus::RadioOn
                || menu_item.check_status == CheckStatus::RadioOff
            {
                create_radio_menu_item().upcast::<gtk::MenuItem>()
            } else {
                panic!("Invalid item check status")
            };
            let weak = self.weak_self.borrow().clone();
            res.connect_activate(move |item| {
                if let Some(s) = weak.upgrade() {
                    if !s.ignore_activate.get() {
                        s.menu_item_selected(item);
                    }
                }
            });
            self.update_menu_item(&res, menu_item, menu_manager);
            res
        };
        res.show();
        res
    }

    fn menu_item_selected(&self, menu_item: &gtk::MenuItem) {
        if menu_item.submenu().is_some() {
            return; // not interested in submenus
        }
        let id_to_menu_item = self.id_to_menu_item.borrow();

        let menu = Self::top_level_menu(&self.menu);
        if let Some(platform_menu) = Self::platform_menu_from_gtk_menu(&menu) {
            platform_menu.item_selected.replace(true);
        }

        let entry = id_to_menu_item.iter().find(|e| e.1 == menu_item);
        if let Some(entry) = entry {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.borrow().on_menu_action(self.handle, *entry.0);
            }
        }
    }

    fn top_level_menu(menu: &gtk::Menu) -> gtk::Menu {
        let mut res = menu.clone();

        loop {
            let widget = res
                .attach_widget()
                .and_then(|w| w.parent())
                .and_then(|w| w.downcast::<gtk::Menu>().ok());
            match widget {
                Some(widget) => {
                    res = widget;
                }
                None => {
                    break;
                }
            }
        }

        res
    }

    // Convert & mnemonics to _
    #[allow(clippy::branches_sharing_code)]
    fn convert_mnemonics(title: &str) -> String {
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
            if mnemonic {
                res.write_char('_').unwrap();
                mnemonic = false;
            }
            res.write_char(c).unwrap();

            if c == '_' {
                res.write_char('_').unwrap();
            }
        }
        res
    }

    fn accelerator_label_code(accelerator: &Accelerator) -> i32 {
        let label = accelerator.label.to_lowercase();
        let value = match label.as_str() {
            // these must match label descriptions from accelerators.dart
            "f1" => gdk_sys::GDK_KEY_F1,
            "f2" => gdk_sys::GDK_KEY_F2,
            "f3" => gdk_sys::GDK_KEY_F3,
            "f4" => gdk_sys::GDK_KEY_F4,
            "f5" => gdk_sys::GDK_KEY_F5,
            "f6" => gdk_sys::GDK_KEY_F6,
            "f7" => gdk_sys::GDK_KEY_F7,
            "f8" => gdk_sys::GDK_KEY_F8,
            "f9" => gdk_sys::GDK_KEY_F9,
            "f10" => gdk_sys::GDK_KEY_F10,
            "f11" => gdk_sys::GDK_KEY_F11,
            "f12" => gdk_sys::GDK_KEY_F12,
            "home" => gdk_sys::GDK_KEY_Home,
            "end" => gdk_sys::GDK_KEY_End,
            "insert" => gdk_sys::GDK_KEY_Insert,
            "delete" => gdk_sys::GDK_KEY_Delete,
            "backspace" => gdk_sys::GDK_KEY_BackSpace,
            "page up" => gdk_sys::GDK_KEY_Page_Up,
            "page down" => gdk_sys::GDK_KEY_Page_Down,
            "space" => gdk_sys::GDK_KEY_space,
            "tab" => gdk_sys::GDK_KEY_Tab,
            "enter" => gdk_sys::GDK_KEY_KP_Enter,
            "arrow up" => gdk_sys::GDK_KEY_Up,
            "arrow down" => gdk_sys::GDK_KEY_Down,
            "arrow left" => gdk_sys::GDK_KEY_Left,
            "arrow right" => gdk_sys::GDK_KEY_Right,
            _ => label.chars().next().unwrap_or(0 as char) as i32,
        };
        value
    }

    fn accelerator_modifier_type(accelerator: &Accelerator) -> ModifierType {
        let mut res = ModifierType::empty();
        if accelerator.alt {
            res |= ModifierType::MOD1_MASK;
        }
        if accelerator.meta {
            res |= ModifierType::META_MASK;
        }
        if accelerator.control {
            res |= ModifierType::CONTROL_MASK;
        }
        if accelerator.shift {
            res |= ModifierType::SHIFT_MASK;
        }
        res
    }

    fn update_menu_item(
        &self,
        item: &gtk::MenuItem,
        menu_item: &MenuItem,
        menu_manager: &MenuManager,
    ) {
        item.set_label(&Self::convert_mnemonics(&menu_item.title));

        let label = item
            .child()
            .and_then(|c| c.downcast::<AccelLabel>().ok())
            .unwrap();

        match &menu_item.accelerator {
            Some(accelerator) => {
                label.set_accel(
                    Self::accelerator_label_code(accelerator) as u32,
                    Self::accelerator_modifier_type(accelerator),
                );
            }
            None => {
                label.set_accel(0, ModifierType::empty());
            }
        }

        item.set_use_underline(true);

        if let Some(check_menu_item) = item.downcast_ref::<gtk::CheckMenuItem>() {
            self.ignore_activate.replace(true);
            check_menu_item_set_checked(
                check_menu_item,
                menu_item.check_status == CheckStatus::CheckOn,
            );
            self.ignore_activate.replace(false);
        }

        if let Some(radio_menu_item) = item.downcast_ref::<gtk::RadioMenuItem>() {
            self.ignore_activate.replace(true);
            radio_menu_item_set_checked(
                radio_menu_item,
                menu_item.check_status == CheckStatus::RadioOn,
            );
            self.ignore_activate.replace(false);
        }

        if let Some(submenu) = menu_item
            .submenu
            .and_then(|s| menu_manager.get_platform_menu(s).ok())
        {
            item.set_submenu(Some(&submenu.menu));
        } else {
            item.set_submenu(None::<&gtk::Menu>);
        }

        item.set_sensitive(menu_item.enabled);
    }

    fn can_update(old_item: &MenuItem, new_item: &MenuItem) -> bool {
        #[derive(PartialEq)]
        enum MenuItemType {
            Separator,
            Regular,
            CheckBox,
            Radio,
        }
        fn get_menu_item_type(item: &MenuItem) -> MenuItemType {
            if item.separator {
                return MenuItemType::Separator;
            }
            match item.check_status {
                CheckStatus::None => MenuItemType::Regular,
                CheckStatus::CheckOn => MenuItemType::CheckBox,
                CheckStatus::CheckOff => MenuItemType::CheckBox,
                CheckStatus::RadioOn => MenuItemType::Radio,
                CheckStatus::RadioOff => MenuItemType::Radio,
            }
        }

        // can't change separator item to non separator or regular/radio/checkbox
        get_menu_item_type(old_item) == get_menu_item_type(new_item)
    }

    fn on_move_current(&self, direction: MenuDirectionType) {
        if direction == MenuDirectionType::Parent && Self::top_level_menu(&self.menu) == self.menu {
            self.move_to_previous_menu();
        } else if direction == MenuDirectionType::Child {
            let selected = self
                .menu
                .selected_item()
                .and_then(|w| w.downcast::<gtk::MenuItem>().ok());
            if let Some(selected) = selected {
                if selected.submenu().is_none() {
                    self.move_to_next_menu();
                }
            }
        }
    }

    pub fn move_to_previous_menu(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.borrow().move_to_previous_menu(self.handle);
        }
    }

    pub fn move_to_next_menu(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.borrow().move_to_next_menu(self.handle);
        }
    }
}

pub struct PlatformMenuManager {}

impl PlatformMenuManager {
    pub fn new(_context: Context) -> Self {
        Self {}
    }

    pub(crate) fn assign_weak_self(&self, _weak_self: Weak<PlatformMenuManager>) {}

    pub fn set_app_menu(&self, _menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
