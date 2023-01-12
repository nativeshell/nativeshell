use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    mem::size_of,
    rc::{Rc, Weak},
    time::Duration,
};

use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Controls::WM_MOUSELEAVE,
        Input::KeyboardAndMouse::{
            TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_DOWN, VK_LEFT, VK_RIGHT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, EndMenu, GetMenuInfo, GetMenuItemCount, GetMenuItemInfoW, GetParent,
            SendMessageW, SetWindowsHookExW, TrackPopupMenuEx, UnhookWindowsHookEx, HHOOK, HMENU,
            MENUINFO, MENUITEMINFOW, MENU_ITEM_STATE, MFS_DISABLED, MF_MOUSESELECT, MF_POPUP,
            MIIM_STATE, MIM_MENUDATA, MSG, MSGF_MENU, TPMPARAMS, TPM_LEFTALIGN, TPM_RETURNCMD,
            TPM_TOPALIGN, TPM_VERTICAL, WH_MSGFILTER, WM_INITMENUPOPUP, WM_KEYDOWN, WM_KEYUP,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MENUSELECT, WM_MOUSEFIRST, WM_MOUSELAST, WM_PAINT,
            WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYUP, WM_USER,
        },
    },
};

use crate::shell::{
    api_model::{PopupMenuRequest, PopupMenuResponse},
    Context, IPoint, IRect, MenuHandle,
};

use super::{
    error::PlatformResult,
    menu::PlatformMenu,
    util::{GET_X_LPARAM, GET_Y_LPARAM, HIWORD, MAKELONG},
    window_base::WindowBaseState,
};

pub trait WindowMenuDelegate {
    fn get_state(&self) -> Ref<WindowBaseState>;
}

pub struct WindowMenu {
    context: Context,
    hwnd: HWND,
    child_hwnd: HWND,
    delegate: Option<Weak<dyn WindowMenuDelegate>>,
    current_menu: RefCell<Option<MenuState>>,
    mouse_state: RefCell<MouseState>,
}

struct MouseState {
    ignore_mouse_leave: bool,
}

struct MenuState {
    platform_menu: Rc<PlatformMenu>,
    request: PopupMenuRequest,
    mouse_in: bool,

    // after pressing left move to previous menu in menubar
    current_item_is_first: bool,

    // after pressing right move to next menu in menubar
    current_item_is_last: bool,

    seen_key_down: bool,
    seen_mouse_down: bool,

    menu_hwnd: HWND,
}

thread_local! {
    static POPUP_PARENT: Cell<HWND> = Cell::new(HWND(0));
}

// Support mouse tracking while popup menu is visible
impl WindowMenu {
    pub fn new(
        context: Context,
        hwnd: HWND,
        child_hwnd: HWND,
        delegate: Weak<dyn WindowMenuDelegate>,
    ) -> Self {
        Self {
            context,
            hwnd,
            child_hwnd,
            delegate: Some(delegate),
            current_menu: RefCell::new(None),
            mouse_state: RefCell::new(MouseState {
                ignore_mouse_leave: false,
            }),
        }
    }

    fn delegate(&self) -> Rc<dyn WindowMenuDelegate> {
        // delegate owns us so unwrap is safe here
        self.delegate.as_ref().and_then(|d| d.upgrade()).unwrap()
    }

    pub fn hide_popup(&self, menu: Rc<PlatformMenu>) {
        if let Some(current_menu) = self.current_menu.borrow().as_ref() {
            if current_menu.platform_menu.handle == menu.handle {
                unsafe {
                    EndMenu();
                }
            }
        }
    }

    pub fn show_popup<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<PopupMenuResponse>) + 'static,
    {
        // We need hook for the tracking rect (if set), but also to forward mouse up
        // because popup menu eats the mouse up message
        let hook = unsafe {
            SetWindowsHookExW(
                WH_MSGFILTER,
                Some(Self::hook_proc),
                HINSTANCE(0),
                GetCurrentThreadId(),
            )
        };

        self.current_menu.borrow_mut().replace(MenuState {
            platform_menu: menu.clone(),
            request: request.clone(),
            mouse_in: false,
            // starting with no item selected, moving left/right moves to next/prev item in menubar
            current_item_is_first: true,
            current_item_is_last: true,
            seen_key_down: false,
            seen_mouse_down: false,
            menu_hwnd: HWND(0),
        });

        let position = self
            .delegate()
            .get_state()
            .local_to_global(&request.position);

        // with popup menu active, the TrackMouseLeaveEvent in flutter view will be fired on every
        // mouse move; we block this in subclass, only allowing our WM_MOUSELEAVE message synthetized
        // when leaving tracking rect
        self.mouse_state.borrow_mut().ignore_mouse_leave = true;

        let mut params = {
            if let Some(item_rect) = request.item_rect.as_ref() {
                let top_left = self
                    .delegate()
                    .get_state()
                    .local_to_global(&item_rect.top_left());

                let bottom_right = self
                    .delegate()
                    .get_state()
                    .local_to_global(&item_rect.bottom_right());

                Some(TPMPARAMS {
                    cbSize: size_of::<TPMPARAMS>() as u32,
                    rcExclude: RECT {
                        left: top_left.x,
                        top: top_left.y,
                        right: bottom_right.x,
                        bottom: bottom_right.y,
                    },
                })
            } else {
                None
            }
        };

        POPUP_PARENT.with(|parent| {
            parent.set(self.hwnd);
        });

        let res = unsafe {
            let res = TrackPopupMenuEx(
                menu.menu,
                (TPM_LEFTALIGN | TPM_TOPALIGN | TPM_VERTICAL | TPM_RETURNCMD).0,
                position.x,
                position.y,
                self.hwnd,
                match &mut params {
                    Some(params) => params as *mut _,
                    None => std::ptr::null_mut(),
                },
            );

            UnhookWindowsHookEx(hook);

            // hook swallows WM_MOUSELEAVE for flutter view (because it is being fired)
            // repeatedy with popup menu visible, so we need to ensure that there's
            // mouse leave hook in palce
            self.track_mouse_leave();

            res.0
        };

        POPUP_PARENT.with(|parent| {
            parent.set(HWND(0));
        });

        if res > 0 {
            if let Some(delegate) = menu.delegate.upgrade() {
                delegate.borrow().on_menu_action(
                    self.current_menu.borrow().as_ref().unwrap().request.handle,
                    res as i64,
                );
            }
        }

        self.current_menu.borrow_mut().take();
        self.mouse_state.borrow_mut().ignore_mouse_leave = false;
        on_done(Ok(PopupMenuResponse {
            item_selected: res != 0,
        }));
    }

    const WM_MENU_HOOK: u32 = WM_USER;
    const WM_MENU_HWND: u32 = WM_USER + 1; // WPARAM contains menu HWND

    extern "system" fn hook_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        unsafe {
            let ptr = l_param.0 as *const MSG;
            let msg: &MSG = &*ptr;

            if code == MSGF_MENU as i32 {
                // for keydown we need parent hwnd
                let mut parent = GetParent(msg.hwnd);
                if parent.0 == 0 {
                    parent = msg.hwnd;
                }

                if msg.message == WM_PAINT {
                    POPUP_PARENT.with(|parent| {
                        SendMessageW(
                            parent.get(),
                            Self::WM_MENU_HWND,
                            WPARAM(msg.hwnd.0 as usize),
                            LPARAM(0),
                        );
                    });
                }

                SendMessageW(parent, Self::WM_MENU_HOOK, w_param, l_param);
            }
            CallNextHookEx(HHOOK(0), code, w_param, l_param)
        }
    }

    pub fn on_subclass_proc(
        &self,
        _h_wnd: HWND,
        u_msg: u32,
        _w_param: WPARAM,
        _l_param: LPARAM,
    ) -> Option<LRESULT> {
        let mouse_state = self.mouse_state.borrow_mut();

        if u_msg == WM_MOUSELEAVE && mouse_state.ignore_mouse_leave {
            return Some(LRESULT(0));
        }
        None
    }

    unsafe fn preselect_first_enabled_item(menu_hwnd: HWND, menu: HMENU) {
        for i in 0..GetMenuItemCount(menu) {
            SendMessageW(menu_hwnd, WM_KEYDOWN, WPARAM(VK_DOWN.0 as usize), LPARAM(0));
            let mut item_info = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STATE,
                ..Default::default()
            };

            GetMenuItemInfoW(menu, i as u32, true, &mut item_info as *mut _);
            if item_info.fState & MFS_DISABLED == MENU_ITEM_STATE(0) {
                break;
            }
        }
    }

    fn on_menu_hwnd(&self, menu_hwnd: HWND) {
        let mut menu = self.current_menu.borrow_mut();
        let menu = menu.as_mut();
        if let Some(menu) = menu {
            menu.menu_hwnd = menu_hwnd;
            if menu.request.preselect_first {
                let hmenu = menu.platform_menu.menu;
                if let Some(context) = self.context.get() {
                    context
                        .run_loop
                        .borrow()
                        .schedule_now(move || unsafe {
                            Self::preselect_first_enabled_item(menu_hwnd, hmenu);
                        })
                        .detach();
                }
            }
        }
    }

    fn on_menu_hook(&self, mut msg: MSG) {
        if self.current_menu.borrow().is_none() {
            return;
        }

        let message = msg.message;

        let mut current_menu = RefMut::map(self.current_menu.borrow_mut(), |x| x.as_mut().unwrap());

        if message == WM_LBUTTONDOWN || message == WM_RBUTTONDOWN {
            current_menu.seen_mouse_down = true;
        }

        if message == WM_KEYDOWN {
            current_menu.seen_key_down = true;
        }

        // Forward initial up button and key up to flutter window; Otherwise menu eats the event
        // and flutter keybord / mouse state gets inconsistent

        if (message == WM_LBUTTONUP || message == WM_RBUTTONUP) && !current_menu.seen_mouse_down {
            unsafe {
                SendMessageW(self.child_hwnd, msg.message, msg.wParam, msg.lParam);
            }
        }

        if (message == WM_KEYUP || message == WM_SYSKEYUP) && !current_menu.seen_key_down {
            unsafe {
                SendMessageW(self.child_hwnd, WM_KEYUP, msg.wParam, msg.lParam);
            }
        }

        // mouse global to local coordinates for mouse messages
        if message >= WM_MOUSEFIRST && message <= WM_MOUSELAST {
            let point = IPoint::xy(GET_X_LPARAM(msg.lParam), GET_Y_LPARAM(msg.lParam));

            // FIXME(knopp): is this necesary?
            // let hwnd = unsafe {
            //     WindowFromPoint(POINT {
            //         x: point.x,
            //         y: point.y,
            //     })
            // };
            // only forward mouse events when over flutter view
            // if hwnd != self.child_hwnd {
            //     return;
            // }

            let point = self.delegate().get_state().global_to_local_physical(&point);
            msg.lParam = LPARAM(MAKELONG(point.x as u16, point.y as u16) as isize);

            if let Some(rect) = &current_menu.request.tracking_rect {
                let scaled: IRect = rect
                    .scaled(self.delegate().get_state().get_scaling_factor())
                    .into();
                if scaled.is_inside(&point) {
                    if !current_menu.mouse_in {
                        current_menu.mouse_in = true;
                    }
                    unsafe {
                        SendMessageW(self.child_hwnd, msg.message, msg.wParam, msg.lParam);
                    }
                } else {
                    self.send_mouse_leave(&mut current_menu);
                }
            }
        } else if message == WM_KEYDOWN {
            let key = msg.wParam.0 as u32;

            let (key_prev, key_next) = match self.delegate().get_state().is_rtl() {
                true => (VK_RIGHT.0 as u32, VK_LEFT.0 as u32),
                false => (VK_LEFT.0 as u32, VK_RIGHT.0 as u32),
            };

            if let Some(delegate) = current_menu.platform_menu.delegate.upgrade() {
                if key == key_prev && current_menu.current_item_is_first {
                    delegate
                        .borrow()
                        .move_to_previous_menu(current_menu.platform_menu.handle);
                } else if key == key_next && current_menu.current_item_is_last {
                    delegate
                        .borrow()
                        .move_to_next_menu(current_menu.platform_menu.handle);
                }
            }
        }
    }

    fn send_mouse_leave(&self, current_menu: &mut RefMut<MenuState>) {
        if current_menu.mouse_in {
            current_menu.mouse_in = false;
            self.mouse_state.borrow_mut().ignore_mouse_leave = false;
            unsafe {
                SendMessageW(self.child_hwnd, WM_MOUSELEAVE, WPARAM(1), LPARAM(0));
            }
            self.mouse_state.borrow_mut().ignore_mouse_leave = true;
        }
    }

    unsafe fn track_mouse_leave(&self) {
        let hwnd = self.child_hwnd;
        if let Some(context) = self.context.get() {
            context
                .run_loop
                .borrow()
                .schedule(
                    // this needs to be delayed a bit, if we schedule it immediately after
                    // hiding popup menu windows will fire WM_MOUSELEAVE even if cursor
                    // is within child_hwnd.
                    Duration::from_millis(50),
                    move || {
                        let mut event = TRACKMOUSEEVENT {
                            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        TrackMouseEvent(&mut event as *mut _);
                    },
                )
                .detach();
        }
    }

    pub fn on_menu_select(&self, _msg: u32, w_param: WPARAM, l_param: LPARAM) {
        if self.current_menu.borrow().is_none() {
            return;
        }

        let mut current_menu = RefMut::map(self.current_menu.borrow_mut(), |x| x.as_mut().unwrap());

        let menu = HMENU(l_param.0);
        let flags = HIWORD(w_param.0 as u32) as u32;

        current_menu.current_item_is_first = menu == current_menu.platform_menu.menu;

        // Mimic behavior of windows menubar; element either has no menu, or it is mouse selected
        // but not highlighted (through keyboard focus)
        current_menu.current_item_is_last =
            flags & MF_POPUP.0 == 0 || flags & MF_MOUSESELECT.0 == MF_MOUSESELECT.0;
    }

    fn on_init_menu(&self, menu: HMENU) {
        if self.current_menu.borrow().is_none() {
            return;
        }

        let mut info = MENUINFO {
            cbSize: std::mem::size_of::<MENUINFO>() as u32,
            fMask: MIM_MENUDATA,
            ..Default::default()
        };
        unsafe {
            if !GetMenuInfo(menu, &mut info as *mut _).as_bool() {
                return;
            }
        }

        let current_menu = Ref::map(self.current_menu.borrow(), |x| x.as_ref().unwrap());

        let handle = MenuHandle(info.dwMenuData as i64);
        if let Some(delegate) = current_menu.platform_menu.delegate.upgrade() {
            delegate.borrow().on_menu_open(handle);
        }
    }

    pub fn handle_message(
        &self,
        _h_wnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_INITMENUPOPUP => {
                self.on_init_menu(HMENU(w_param.0 as isize));
            }
            WM_MENUSELECT => {
                self.on_menu_select(msg, w_param, l_param);
            }
            Self::WM_MENU_HOOK => {
                let ptr = l_param.0 as *const MSG;
                let msg: &MSG = unsafe { &*ptr };
                self.on_menu_hook(*msg);
            }
            Self::WM_MENU_HWND => {
                self.on_menu_hwnd(HWND(w_param.0 as isize));
            }
            _ => {}
        }
        None
    }
}
