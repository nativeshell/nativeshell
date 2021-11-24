use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use windows::Win32::{
    Foundation::PWSTR,
    Graphics::Gdi::{HBITMAP, HBRUSH},
    UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, InsertMenuItemW, RemoveMenu, SetMenuInfo, SetMenuItemInfoW,
        HMENU, MENUINFO, MENUINFO_STYLE, MENUITEMINFOW, MFS_CHECKED, MFS_DISABLED, MFS_ENABLED,
        MFT_RADIOCHECK, MFT_SEPARATOR, MFT_STRING, MF_BYCOMMAND, MIIM_FTYPE, MIIM_ID, MIIM_STATE,
        MIIM_STRING, MIIM_SUBMENU, MIM_MENUDATA, MIM_STYLE,
    },
};

use super::{
    error::{PlatformError, PlatformResult},
    util::to_utf16,
};

use crate::{
    shell::{
        api_model::{CheckStatus, Menu, MenuItem},
        Context, MenuDelegate, MenuHandle, MenuManager,
    },
    util::{update_diff, DiffResult},
};

pub struct PlatformMenu {
    pub(super) handle: MenuHandle,
    pub(super) menu: HMENU,
    previous_menu: RefCell<Menu>,
    weak_self: RefCell<Weak<PlatformMenu>>,
    pub(super) delegate: Weak<RefCell<dyn MenuDelegate>>,
}

pub struct PlatformMenuManager {}

impl PlatformMenuManager {
    pub fn new(_context: Context) -> Self {
        Self {}
    }

    pub(crate) fn assign_weak_self(&self, _weak_self: Weak<PlatformMenuManager>) {}

    pub fn set_app_menu(&self, _menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }
}

impl PlatformMenu {
    pub fn new(
        _context: Context,
        handle: MenuHandle,
        delegate: Weak<RefCell<dyn MenuDelegate>>,
    ) -> Self {
        let menu = unsafe {
            let menu = CreatePopupMenu();

            let mut info = MENUINFO {
                cbSize: std::mem::size_of::<MENUINFO>() as u32,
                fMask: (MIM_MENUDATA | MIM_STYLE),
                dwStyle: MENUINFO_STYLE(0),
                cyMax: 0,
                hbrBack: HBRUSH(0),
                dwContextHelpID: 0,
                dwMenuData: handle.0 as usize,
            };

            SetMenuInfo(menu, &mut info as *mut _);
            menu
        };
        Self {
            handle,
            menu,
            previous_menu: RefCell::new(Default::default()),
            weak_self: RefCell::new(Weak::new()),
            delegate,
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformMenu>) {
        *self.weak_self.borrow_mut() = weak;
    }

    pub fn get_menu_item_info(
        item: &MenuItem,
        title: &[u16],
        manager: &MenuManager,
    ) -> MENUITEMINFOW {
        let submenu = item
            .submenu
            .and_then(|h| manager.get_platform_menu(h).ok())
            .map(|m| m.menu)
            .unwrap_or_else(|| HMENU(0));

        let mut item_type = {
            match item.separator {
                true => MFT_SEPARATOR,
                false => MFT_STRING,
            }
        };

        let mut state = MFS_ENABLED;
        if !item.enabled {
            state |= MFS_DISABLED;
        }
        if item.check_status == CheckStatus::CheckOn {
            state |= MFS_CHECKED;
        }
        if item.check_status == CheckStatus::RadioOn {
            item_type |= MFT_RADIOCHECK;
            state |= MFS_CHECKED;
        }

        MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE | MIIM_ID | MIIM_STATE | MIIM_STRING | MIIM_SUBMENU,
            fType: item_type,
            fState: state,
            wID: item.id as u32,
            hSubMenu: submenu,
            hbmpChecked: HBITMAP(0),
            hbmpUnchecked: HBITMAP(0),
            dwItemData: 0,
            dwTypeData: PWSTR(title.as_ptr() as *mut _),
            cch: title.len() as u32,
            hbmpItem: HBITMAP(0),
        }
    }

    pub fn update_from_menu(&self, menu: Menu, manager: &MenuManager) -> PlatformResult<()> {
        let mut previous_menu = self.previous_menu.borrow_mut();

        let diff = update_diff(&previous_menu.items, &menu.items, |a, b| {
            Self::can_update(a, b)
        });

        // First remove items for menu;
        let diff: Vec<_> = diff
            .iter()
            .filter(|res| match res {
                DiffResult::Remove(res) => {
                    unsafe {
                        RemoveMenu(self.menu, res.id as u32, MF_BYCOMMAND);
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
                    let title = to_utf16(&self.title_for_item(new));
                    let mut info = Self::get_menu_item_info(new, &title, manager);
                    unsafe {
                        SetMenuItemInfoW(self.menu, old.id as u32, false, &mut info as *mut _);
                    }
                }
                DiffResult::Insert(item) => {
                    let title = to_utf16(&self.title_for_item(item));
                    let mut info = Self::get_menu_item_info(item, &title, manager);
                    unsafe {
                        InsertMenuItemW(self.menu, i as u32, true, &mut info as *mut _);
                    }
                }
            }
        }

        *previous_menu = menu;

        Ok(())
    }

    fn title_for_item(&self, item: &MenuItem) -> String {
        let mut res = item.title.clone();
        if let Some(accelerator) = &item.accelerator {
            let mut separator = '\t';

            if accelerator.control {
                res.push(separator);
                res.push_str("Ctrl");
                separator = '+';
            }
            if accelerator.alt {
                res.push(separator);
                res.push_str("Alt");
                separator = '+';
            }
            if accelerator.shift {
                res.push(separator);
                res.push_str("Shift");
                separator = '+';
            }
            if accelerator.meta {
                res.push(separator);
                res.push_str("Win");
                separator = '+';
            }
            res.push(separator);
            res.push_str(&accelerator.label);
        }
        res
    }

    fn can_update(_old_item: &MenuItem, _new_item: &MenuItem) -> bool {
        true
    }
}

impl Drop for PlatformMenu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.menu);
        }
    }
}
