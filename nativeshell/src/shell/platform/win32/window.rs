use std::{
    cell::{Cell, Ref, RefCell},
    ptr::null_mut,
    rc::{Rc, Weak},
    time::Duration,
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled, SetFocus},
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{
            DefWindowProcW, EndMenu, GetClientRect, GetSystemMenu, MoveWindow, SendMessageW,
            SetForegroundWindow, SetParent, TrackPopupMenuEx, GWL_HWNDPARENT, MSG, SIZE_MAXIMIZED,
            SIZE_MINIMIZED, TPM_RETURNCMD, WA_ACTIVE, WA_CLICKACTIVE, WM_ACTIVATE,
            WM_DISPLAYCHANGE, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCCALCSIZE,
            WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETFOCUS, WM_SHOWWINDOW, WM_SIZE, WM_SYSCOMMAND,
        },
    },
};

use crate::{
    codec::Value,
    shell::{
        api_model::{
            BoolTransition, DragEffect, DragRequest, PopupMenuRequest, PopupMenuResponse,
            WindowCollectionBehavior, WindowGeometry, WindowGeometryFlags, WindowGeometryRequest,
            WindowStateFlags, WindowStyle,
        },
        Context, IPoint, PlatformWindowDelegate, Point,
    },
    util::LateRefCell,
};

use super::{
    drag_context::DragContext,
    dxgi_hook::{set_override_parent_hwnd, take_override_parent_hwnd},
    engine::PlatformEngine,
    error::{PlatformError, PlatformResult},
    flutter_sys::*,
    menu::PlatformMenu,
    screen_manager::PlatformScreenManager,
    window_adapter::{SetWindowLongPtrW, WindowAdapter},
    window_base::{WindowBaseState, WindowDelegate},
    window_menu::{WindowMenu, WindowMenuDelegate},
};

pub type PlatformWindowType = isize; // HWND

pub struct PlatformWindow {
    context: Context,
    hwnd: Cell<HWND>,
    child_hwnd: Cell<HWND>,
    state: LateRefCell<WindowBaseState>,
    window_menu: LateRefCell<WindowMenu>,
    drag_context: LateRefCell<Rc<DragContext>>,
    weak_self: LateRefCell<Weak<PlatformWindow>>,
    parent: Option<Rc<PlatformWindow>>,
    modal_child: Cell<Option<HWND>>,
    flutter_controller: LateRefCell<FlutterDesktopViewControllerRef>,
    delegate: Weak<dyn PlatformWindowDelegate>,
    modal_close_callback: RefCell<Option<Box<dyn FnOnce(PlatformResult<Value>)>>>,
    ready_to_show: Cell<bool>,
    show_when_ready: Cell<bool>,
    mouse_state: RefCell<MouseState>,
    window_state_flags: RefCell<WindowStateFlags>,
}

struct MouseState {
    // last button down message used to synthetize button up when displaying menu
    last_button_down: Option<MSG>,
}

impl PlatformWindow {
    pub fn new(
        context: Context,
        delegate: Weak<dyn PlatformWindowDelegate>,
        parent: Option<Rc<PlatformWindow>>,
    ) -> Self {
        PlatformWindow {
            context,
            hwnd: Cell::new(HWND(0)),
            child_hwnd: Cell::new(HWND(0)),
            state: LateRefCell::new(),
            window_menu: LateRefCell::new(),
            drag_context: LateRefCell::new(),
            parent,
            modal_child: Cell::new(None),
            weak_self: LateRefCell::new(),
            flutter_controller: LateRefCell::new(),
            delegate,
            modal_close_callback: RefCell::new(None),
            ready_to_show: Cell::new(false),
            show_when_ready: Cell::new(false),
            mouse_state: RefCell::new(MouseState {
                last_button_down: None,
            }),
            window_state_flags: RefCell::new(WindowStateFlags::default()),
        }
    }

    pub fn layout_child(&self) {
        unsafe {
            let mut rect: RECT = RECT::default();
            GetClientRect(self.hwnd(), &mut rect as *mut _);

            MoveWindow(
                self.child_hwnd(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                true,
            );
        }
    }

    pub fn get_platform_window(&self) -> PlatformWindowType {
        self.hwnd().0
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformWindow>, engine: &PlatformEngine) {
        self.weak_self.set(weak.clone());

        let win = self.create_window("");
        self.hwnd.set(win);

        self.state.set(WindowBaseState::new(win, weak.clone()));

        unsafe {
            // Flutter will attempt to create surface during initialization, but we need the
            // composition target to be flutter view parent for smoother resizing; This is done
            // by hooking IDCompositionDevice::CreateTargetForHwnd; However when flutter calls it,
            // the flutter view will not have parent set yet, so we need to provide it here
            set_override_parent_hwnd(win);

            self.flutter_controller
                .set(FlutterDesktopViewControllerCreate(1, 1, engine.handle));

            let view = FlutterDesktopViewControllerGetView(*self.flutter_controller.borrow());
            self.child_hwnd
                .set(HWND(FlutterDesktopViewGetHWND(view) as _));

            // remove parent override, just in case
            take_override_parent_hwnd();

            SetParent(self.child_hwnd(), self.hwnd());

            // intercept flutter messages
            SetWindowSubclass(
                self.child_hwnd.get(),
                Some(Self::subclass_proc),
                self as *const _ as usize,
                0,
            );
        }

        self.window_menu.set(WindowMenu::new(
            self.context.clone(),
            win,
            self.child_hwnd(),
            weak.clone(),
        ));

        if let Some(context) = self.context.get() {
            let drag_context = Rc::new(DragContext::new(&context, weak));
            self.drag_context.set(drag_context.clone());
            drag_context.assign_weak_self(Rc::downgrade(&drag_context));
        }
    }
}

impl WindowAdapter for PlatformWindow {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        let res = if self.state.is_set() {
            self.handle_message(h_wnd, msg, w_param, l_param)
        } else {
            None
        };
        match res {
            Some(res) => res,
            None => self.default_wnd_proc(h_wnd, msg, w_param, l_param),
        }
    }
}

impl WindowDelegate for PlatformWindow {
    fn should_close(&self) {
        let u = self.delegate.upgrade();
        if let Some(u) = u {
            u.did_request_close();
        }
    }

    fn will_close(&self) {
        let callback = self.modal_close_callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback(Ok(Value::Null));
        }

        let u = self.delegate.upgrade();
        if let Some(u) = u {
            u.will_close();
        }
    }
}

impl WindowMenuDelegate for PlatformWindow {
    fn get_state(&self) -> Ref<WindowBaseState> {
        self.state.borrow()
    }
}

impl PlatformWindow {
    pub fn hwnd(&self) -> HWND {
        self.hwnd.get()
    }

    pub fn child_hwnd(&self) -> HWND {
        self.child_hwnd.get()
    }

    fn show_internal(&self) -> PlatformResult<()> {
        unsafe {
            // SetForegroundWindow(self.hwnd());
            // let style = GetWindowLongW(self.hwnd(), GWL_EXSTYLE) as i32;
            // SetWindowLongW(self.hwnd(), GWL_EXSTYLE, style & !WS_EX_NOACTIVATE);
            SetFocus(self.child_hwnd());
        }
        let delegate = self.delegate.clone();
        self.state.borrow().show(move || {
            if let Some(delegate) = delegate.upgrade() {
                delegate.visibility_changed(true);
            }
        })
    }

    pub fn show(&self) -> PlatformResult<()> {
        if self.ready_to_show.get() {
            self.show_internal()
        } else {
            self.show_when_ready.set(true);
            Ok(())
        }
    }

    pub fn hide(&self) -> PlatformResult<()> {
        if self.ready_to_show.get() {
            self.state.borrow().hide()?;
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.visibility_changed(false);
            }
        } else {
            self.show_when_ready.set(false);
        }
        Ok(())
    }

    pub fn activate(&self, _activate_application: bool) -> PlatformResult<bool> {
        self.state.borrow().activate()
    }

    pub fn deactivate(&self, _deactivate_application: bool) -> PlatformResult<bool> {
        self.state.borrow().deactivate()
    }

    pub fn ready_to_show(&self) -> PlatformResult<()> {
        self.ready_to_show.set(true);
        if self.show_when_ready.get() {
            self.show_internal()
        } else {
            Ok(())
        }
    }

    pub fn close(&self) -> PlatformResult<()> {
        self.drag_context.borrow().shut_down()?;

        // There shouldn't be any way to close the window without calling
        // PlatformWindow::close so it shoud be safe to enable parent here; We
        // could do it in will_close (which gets posted as notification when HWND)
        // gets destroyed, but that's too late and causes flicker
        if let Some(parent) = &self.parent {
            if parent.modal_child.get() == Some(self.hwnd()) {
                unsafe {
                    EnableWindow(parent.hwnd(), true);
                    parent.modal_child.get().take();
                    SetForegroundWindow(parent.hwnd());
                }
            }
        }

        self.state.borrow().close()
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        self.state.borrow().set_geometry(geometry)
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        self.state.borrow().get_geometry()
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        self.state.borrow().supported_geometry()
    }

    pub fn set_title(&self, title: String) -> PlatformResult<()> {
        self.state.borrow().set_title(title)
    }

    pub fn set_collection_behavior(
        &self,
        _behavior: WindowCollectionBehavior,
    ) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn set_minimized(&self, minimized: bool) -> PlatformResult<()> {
        if minimized && !self.window_state_flags.borrow().is_minimized() {
            self.state.borrow().minimize();
        } else if !minimized && self.window_state_flags.borrow().is_minimized() {
            self.state.borrow().restore();
        }
        Ok(())
    }

    pub fn set_maximized(&self, maximized: bool) -> PlatformResult<()> {
        if maximized && !self.window_state_flags.borrow().is_maximized() {
            self.state.borrow().maximize();
        } else if !maximized && self.window_state_flags.borrow().is_maximized() {
            self.state.borrow().restore();
        }
        Ok(())
    }

    pub fn set_full_screen(&self, _full_screen: bool) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn get_screen_id(&self) -> PlatformResult<i64> {
        PlatformScreenManager::screen_id_from_hwnd(self.hwnd())
    }

    pub fn save_position_to_string(&self) -> PlatformResult<String> {
        self.state.borrow().save_position_to_string()
    }

    pub fn restore_position_from_string(&self, position: String) -> PlatformResult<()> {
        self.state.borrow().restore_position_from_string(position)
    }

    pub fn get_window_state_flags(&self) -> PlatformResult<WindowStateFlags> {
        Ok(self.window_state_flags.borrow().clone())
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        self.state.borrow().set_style(style)?;
        self.force_redraw();
        Ok(())
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        self.state.borrow().perform_window_drag()
    }

    pub fn is_enabled(&self) -> bool {
        unsafe { IsWindowEnabled(self.hwnd()).as_bool() }
    }

    pub fn show_modal<F>(&self, done_callback: F)
    where
        F: FnOnce(PlatformResult<Value>) + 'static,
    {
        self.modal_close_callback
            .borrow_mut()
            .replace(Box::new(done_callback));

        match &self.parent {
            Some(parent) => {
                let hwnd = Some(self.hwnd());
                parent.modal_child.set(hwnd);
                unsafe {
                    EnableWindow(parent.hwnd(), false);
                    SetWindowLongPtrW(self.hwnd(), GWL_HWNDPARENT.0, parent.hwnd().0);
                }
            }
            None => {}
        }
        if let Err(error) = self.show() {
            let cb = self.modal_close_callback.borrow_mut().take();
            if let Some(cb) = cb {
                cb(Err(error));
            }
        }
    }

    pub fn close_with_result(&self, result: Value) -> PlatformResult<()> {
        let callback = self.modal_close_callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback(Ok(result));
        }
        self.close()
    }

    pub fn show_popup_menu<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<PopupMenuResponse>) + 'static,
    {
        let weak = self.weak_self.clone_value();

        unsafe {
            EndMenu();
        }

        if let Some(context) = self.context.get() {
            context
                .run_loop
                .borrow()
                .schedule_now(move || {
                    let this = weak.upgrade();
                    if let Some(this) = this {
                        this.window_menu.borrow().show_popup(menu, request, on_done);
                    }
                })
                .detach();
        }
    }

    pub fn hide_popup_menu(&self, menu: Rc<PlatformMenu>) -> PlatformResult<()> {
        self.window_menu.borrow().hide_popup(menu);
        Ok(())
    }

    pub fn show_system_menu(&self) -> PlatformResult<()> {
        let menu = unsafe { GetSystemMenu(self.hwnd(), false) };
        let position = self.get_state().local_to_global(&Point::xy(0.0, 0.0));
        let hwnd = self.hwnd();
        if let Some(context) = self.context.get() {
            context
                .run_loop
                .borrow()
                .schedule_now(move || unsafe {
                    let cmd = TrackPopupMenuEx(
                        menu,
                        TPM_RETURNCMD.0,
                        position.x,
                        position.y,
                        hwnd,
                        null_mut(),
                    );
                    SendMessageW(hwnd, WM_SYSCOMMAND, WPARAM(cmd.0 as usize), LPARAM(0));
                })
                .detach();
        }
        Ok(())
    }

    pub fn set_window_menu(&self, _menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn begin_drag_session(&self, request: DragRequest) -> PlatformResult<()> {
        self.drag_context.borrow().begin_drag_session(request)?;

        self.synthetize_mouse_up();
        Ok(())
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {
        self.drag_context.borrow().set_pending_effect(effect);
    }

    pub fn delegate(&self) -> Option<Rc<dyn PlatformWindowDelegate>> {
        self.delegate.upgrade()
    }

    pub fn global_to_local(&self, offset: &IPoint) -> Point {
        self.state.borrow().global_to_local(offset)
    }

    pub fn local_to_global(&self, offset: Point) -> IPoint {
        self.state.borrow().local_to_global(&offset)
    }

    fn force_redraw(&self) {
        unsafe {
            FlutterDesktopViewControllerForceRedraw(*self.flutter_controller.borrow());
        }
    }

    fn update_state_flags(&self, new_state_flags: WindowStateFlags) {
        if *self.window_state_flags.borrow() != new_state_flags {
            self.window_state_flags.replace(new_state_flags);
            if let Some(delegate) = self.delegate() {
                delegate.state_flags_changed();
            }
        }
    }

    fn on_wmsize(&self, w_param: WPARAM) {
        let mut new_state_flags = self.window_state_flags.borrow().clone();
        new_state_flags.maximized = if w_param.0 as u32 & SIZE_MAXIMIZED != 0 {
            BoolTransition::Yes
        } else {
            BoolTransition::No
        };
        new_state_flags.minimized = if w_param.0 as u32 & SIZE_MINIMIZED != 0 {
            BoolTransition::Yes
        } else {
            BoolTransition::No
        };
        self.update_state_flags(new_state_flags);
    }

    fn on_wmactivate(&self, w_param: WPARAM) {
        let mut new_state_flags = self.window_state_flags.borrow().clone();
        new_state_flags.active = w_param.0 as u32 & (WA_ACTIVE | WA_CLICKACTIVE) != 0;
        self.update_state_flags(new_state_flags);
    }

    fn handle_message(
        &self,
        h_wnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> Option<LRESULT>
    where
        Self: Sized,
    {
        match msg {
            WM_SIZE => {
                self.layout_child();
                self.on_wmsize(w_param);
            }
            WM_SHOWWINDOW => {
                self.layout_child();
                self.force_redraw();
            }
            WM_DISPLAYCHANGE => {
                unsafe {
                    SendMessageW(self.child_hwnd(), WM_SHOWWINDOW, WPARAM(1), LPARAM(1));
                }
                let hwnd = self.child_hwnd();
                if let Some(context) = self.context.get() {
                    context
                        .run_loop
                        .borrow()
                        .schedule(Duration::from_secs(1), move || unsafe {
                            SendMessageW(hwnd, WM_SHOWWINDOW, WPARAM(1), LPARAM(1));
                        })
                        .detach();
                }
            }
            _ => {}
        }
        if self.flutter_controller.is_set() {
            unsafe {
                let mut lresult: i64 = Default::default();
                if FlutterDesktopViewControllerHandleTopLevelWindowProc(
                    *self.flutter_controller.borrow(),
                    h_wnd.0 as _,
                    msg,
                    w_param.0 as _,
                    l_param.0 as _,
                    &mut lresult as *mut _,
                ) {
                    return Some(LRESULT(lresult as _));
                }
            }
        }

        match msg {
            WM_SETFOCUS => unsafe {
                SetFocus(self.child_hwnd());
            },
            WM_ACTIVATE => {
                self.on_wmactivate(w_param);
            }
            WM_NCCALCSIZE => unsafe {
                // No redirection surface, or redireciton surface with removed border; In this case we
                // need to resize child in WM_NCALCSIZE for better performance
                if w_param.0 == 1
                    && (!self.get_state().has_redirection_surface()
                        || self.get_state().remove_border())
                {
                    // if there is border, run default proc to determine border
                    let res = if !self.get_state().remove_border() {
                        Some(DefWindowProcW(h_wnd, msg, w_param, l_param))
                    } else {
                        None
                    };
                    let rect: &RECT = &*(l_param.0 as *const RECT);

                    MoveWindow(
                        self.child_hwnd(),
                        0,
                        0,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        true,
                    );

                    if res.is_some() {
                        return res;
                    }
                }
            },
            WM_EXITSIZEMOVE => {
                self.force_redraw();
            }
            _ => {}
        }

        if self.window_menu.is_set() {
            let res = self
                .window_menu
                .borrow()
                .handle_message(h_wnd, msg, w_param, l_param);
            if res.is_some() {
                return res;
            }
        }

        if self.state.is_set() {
            self.state
                .borrow()
                .handle_message(h_wnd, msg, w_param, l_param)
        } else {
            None
        }
    }

    pub fn synthetize_mouse_up(&self) {
        // synthetize mouse up / down event
        let mouse_msg = self.mouse_state.borrow_mut().last_button_down.take();
        if let Some(mut mouse_msg) = mouse_msg {
            if mouse_msg.message == WM_LBUTTONDOWN {
                mouse_msg.message = WM_LBUTTONUP;
            } else if mouse_msg.message == WM_RBUTTONDOWN {
                mouse_msg.message = WM_RBUTTONUP;
            }
            unsafe {
                SendMessageW(
                    self.child_hwnd.get(),
                    mouse_msg.message,
                    mouse_msg.wParam,
                    mouse_msg.lParam,
                );
            }
        }
    }

    fn on_subclass_proc(
        &self,
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        {
            let mut mouse_state = self.mouse_state.borrow_mut();

            if u_msg == WM_LBUTTONDOWN || u_msg == WM_RBUTTONDOWN {
                mouse_state.last_button_down.replace(MSG {
                    hwnd: h_wnd,
                    message: u_msg,
                    wParam: w_param,
                    lParam: l_param,
                    ..Default::default()
                });
            } else if u_msg == WM_LBUTTONUP || u_msg == WM_RBUTTONUP {
                mouse_state.last_button_down.take();
            }

            let r = self
                .window_menu
                .borrow()
                .on_subclass_proc(h_wnd, u_msg, w_param, l_param);
            if let Some(r) = r {
                return r;
            }

            let r = self
                .state
                .borrow()
                .handle_child_message(h_wnd, u_msg, w_param, l_param);
            if let Some(r) = r {
                return r;
            }
        }

        unsafe { DefSubclassProc(h_wnd, u_msg, w_param, l_param) }
    }

    extern "system" fn subclass_proc(
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
        u_id_subclass: usize,
        _dw_ref_data: usize,
    ) -> LRESULT {
        unsafe {
            let win: &PlatformWindow = &*(u_id_subclass as *const PlatformWindow);
            win.on_subclass_proc(h_wnd, u_msg, w_param, l_param)
        }
    }
}
