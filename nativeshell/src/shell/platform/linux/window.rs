use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    time::Duration,
};

use gdk::{Display, Event, EventType, EventWindowState, WMDecoration, WMFunction};
use glib::{Cast, ObjectExt};
use gtk::{
    prelude::{ContainerExt, GtkWindowExt, OverlayExt, WidgetExt},
    propagate_event, EventBox, Inhibit, Overlay, Widget,
};

use crate::{
    codec::Value,
    shell::{
        api_model::{
            BoolTransition, DragEffect, DragRequest, PopupMenuRequest, PopupMenuResponse,
            WindowCollectionBehavior, WindowFrame, WindowGeometry, WindowGeometryFlags,
            WindowGeometryRequest, WindowStateFlags, WindowStyle,
        },
        Context, PlatformWindowDelegate, Point, Size,
    },
    util::{LateRefCell, OkLog},
};

use super::{
    drag_context::{DragContext, DropContext},
    engine::PlatformEngine,
    error::{PlatformError, PlatformResult},
    flutter::View,
    menu::PlatformMenu,
    screen_manager::PlatformScreenManager,
    size_widget::{create_size_widget, size_widget_set_min_size},
    utils::{get_session_type, synthetize_button_up, translate_event_to_window, SessionType},
    window_menu::WindowMenu,
};

pub type PlatformWindowType = gtk::Window;

#[derive(serde::Deserialize, serde::Serialize, Debug, Default, PartialEq, Eq, Clone)]
struct WindowState {
    width: i32,
    height: i32,
    is_minimized: bool,
    is_maximized: bool,
    is_full_screen: bool,
    is_active: bool,
}

pub struct PlatformWindow {
    context: Context,
    pub(super) window: gtk::Window,
    weak_self: LateRefCell<Weak<PlatformWindow>>,
    parent: Option<Weak<PlatformWindow>>,
    pub(super) delegate: Weak<dyn PlatformWindowDelegate>,
    modal_close_callback: RefCell<Option<Box<dyn FnOnce(PlatformResult<Value>)>>>,
    size_widget: Widget,
    pub(super) view: LateRefCell<View>,
    ready_to_show: Cell<bool>,
    show_when_ready: Cell<bool>,
    pending_first_frame: Cell<bool>,
    last_geometry_request: RefCell<GeometryRequest>,
    pending_geometry_request: RefCell<Option<GeometryRequest>>,
    window_size_in_progress: Cell<bool>,
    last_window_style: RefCell<Option<WindowStyle>>,
    pub(super) last_event: RefCell<HashMap<EventType, Event>>,
    deleting: Cell<bool>,
    pub(super) window_menu: LateRefCell<WindowMenu>,
    pub(super) drop_context: LateRefCell<DropContext>,
    drag_context: LateRefCell<DragContext>,
    window_state: RefCell<WindowState>,
}

impl PlatformWindow {
    pub fn new(
        context: Context,
        delegate: Weak<dyn PlatformWindowDelegate>,
        parent: Option<Rc<PlatformWindow>>,
    ) -> Self {
        Self {
            context,
            window: gtk::Window::new(gtk::WindowType::Toplevel),
            weak_self: LateRefCell::new(),
            delegate,
            modal_close_callback: RefCell::new(None),
            parent: parent.map(|p| Rc::downgrade(&p)),
            size_widget: create_size_widget(),
            view: LateRefCell::new(),
            ready_to_show: Cell::new(false),
            show_when_ready: Cell::new(false),
            pending_first_frame: Cell::new(true),
            last_geometry_request: RefCell::new(Default::default()),
            pending_geometry_request: RefCell::new(None),
            window_size_in_progress: Cell::new(false),
            last_window_style: RefCell::new(None),
            last_event: RefCell::new(HashMap::new()),
            deleting: Cell::new(false),
            window_menu: LateRefCell::new(),
            drop_context: LateRefCell::new(),
            drag_context: LateRefCell::new(),
            window_state: RefCell::new(Default::default()),
        }
    }

    pub fn on_first_frame(&self) {
        self.window.set_opacity(1.0);
    }

    pub fn engine_launched(&self) {
        self.window.hide();
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformWindow>, engine: &PlatformEngine) {
        self.weak_self.set(weak.clone());

        self.window_menu.set(WindowMenu::new(weak.clone()));

        let overlay = Overlay::new();
        self.window.add(&overlay);

        overlay.add(&self.size_widget);
        overlay.add_overlay(&self.window_menu.borrow().menu_bar_container);

        self.view.set(engine.view.clone());
        overlay.add_overlay(&self.view.borrow().clone());

        self.view.borrow().grab_focus();

        let weak_clone = weak.clone();
        self.window.connect_size_allocate(move |_, _| {
            if let Some(s) = weak_clone.upgrade() {
                s.on_size_allocate();
            }
        });

        let weak_clone = weak.clone();
        self.window.connect_window_state_event(move |_, state| {
            if let Some(s) = weak_clone.upgrade() {
                s.on_window_state_changed(state);
            }
            Inhibit(false)
        });

        self.window.realize();
        unsafe {
            self.window
                .window()
                .unwrap()
                .set_data("nativeshell_platform_window", weak);
        }

        // by default make window resizable, non resizable window need size
        // specified
        self.window.set_resizable(true);

        let weak = self.weak_self.borrow().clone();
        let weak_clone = weak.clone();
        self.window.connect_delete_event(move |_, _| {
            let s = weak_clone.upgrade();
            if let Some(s) = s {
                s.on_delete()
            } else {
                Inhibit(false)
            }
        });

        if let Some(context) = self.context.get() {
            self.drop_context
                .set(DropContext::new(&context, weak.clone()));
            self.drag_context.set(DragContext::new(&context, weak));
        }
        self.connect_drag_drop_events();

        self.schedule_first_frame_notification();
    }

    fn on_size_allocate(&self) {
        let mut state = self.window_state.borrow_mut();
        if !state.is_maximized && !state.is_full_screen {
            let size = self.window.size();
            state.width = size.0;
            state.height = size.1;
        }
        self.window_size_in_progress.set(false);
        if self.pending_geometry_request.borrow().is_some() {
            // This must be done after Gtk allocation is done, so schedule it on next
            // run loop turn
            let weak_self = self.weak_self.borrow().clone();
            if let Some(context) = self.context.get() {
                context
                    .run_loop
                    .borrow()
                    .schedule_now(move || {
                        if let Some(s) = weak_self.upgrade() {
                            if let Some(req) = s.pending_geometry_request.borrow_mut().take() {
                                s._set_geometry(req, false);
                            }
                        }
                    })
                    .detach();
            }
        }
    }

    fn on_window_state_changed(&self, state: &EventWindowState) {
        let state_flags_changed = {
            let mut window_state = self.window_state.borrow_mut();
            let prev_state = window_state.clone();
            window_state.is_maximized = state
                .new_window_state()
                .contains(gdk::WindowState::MAXIMIZED);
            window_state.is_full_screen = state
                .new_window_state()
                .contains(gdk::WindowState::FULLSCREEN);
            window_state.is_minimized = state
                .new_window_state()
                .contains(gdk::WindowState::ICONIFIED);
            window_state.is_active = state.new_window_state().contains(gdk::WindowState::FOCUSED);
            *window_state != prev_state
        };
        if state_flags_changed {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.state_flags_changed();
            }
        }
    }

    fn connect_drag_drop_events(&self) {
        if let Some(event_box) = self.get_event_box() {
            self.drop_context.borrow().register(&event_box);

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_motion(move |w, context, x, y, time| {
                if let Some(window) = weak.upgrade() {
                    window
                        .drop_context
                        .borrow()
                        .drag_motion(w, context, x, y, time);
                }
                true
            });

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_leave(move |w, context, time| {
                if let Some(window) = weak.upgrade() {
                    window.drop_context.borrow().drag_leave(w, context, time);
                }
            });

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_drop(move |w, context, x, y, time| {
                if let Some(window) = weak.upgrade() {
                    window
                        .drop_context
                        .borrow()
                        .drag_drop(w, context, x, y, time);
                }
                true
            });

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_data_received(move |w, context, x, y, data, info, time| {
                if let Some(window) = weak.upgrade() {
                    window
                        .drop_context
                        .borrow()
                        .drag_data_received(w, context, x, y, data, info, time);
                }
            });

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_data_get(
                move |_w, _context, selection_data, target_info, _time| {
                    if let Some(window) = weak.upgrade() {
                        window
                            .drag_context
                            .borrow()
                            .get_data(selection_data, target_info);
                    }
                },
            );

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_failed(move |_w, _context, _result| {
                if let Some(window) = weak.upgrade() {
                    window.drag_context.borrow().drag_failed();
                }
                Inhibit(false)
            });

            let weak = self.weak_self.borrow().clone();
            event_box.connect_drag_end(move |_e, context| {
                if let Some(window) = weak.upgrade() {
                    window.drag_context.borrow().drag_end(context);
                }
            });
        }
    }

    fn on_delete(&self) -> Inhibit {
        if let Some(delegate) = self.delegate.upgrade() {
            if self.deleting.get() {
                let callback = self.modal_close_callback.borrow_mut().take();
                if let Some(callback) = callback {
                    callback(Ok(Value::Null));
                }
                delegate.will_close();
                return Inhibit(false);
            } else {
                delegate.did_request_close();
                return Inhibit(true);
            }
        }
        Inhibit(false)
    }

    fn get_event_box(&self) -> Option<EventBox> {
        let mut res: Option<EventBox> = None;
        self.view.borrow().forall(|w| {
            let w = w.downcast_ref::<EventBox>();
            if w.is_some() {
                res = w.cloned()
            }
        });
        res
    }

    fn get_gl_area(&self) -> Option<Widget> {
        let mut res: Option<Widget> = None;
        self.view.borrow().forall(|w| {
            if w.type_().name() == "FlGLArea" {
                res = Some(w.clone());
            }
        });
        res
    }

    fn on_draw(&self) {
        if self.pending_first_frame.get()
            && self.ready_to_show.get()
            && self.get_gl_area().is_some()
        {
            self.pending_first_frame.replace(false);
            let weak = self.weak_self.borrow().clone();
            if let Some(context) = self.context.get() {
                context
                    .run_loop
                    .borrow()
                    .schedule(
                        // delay one frame, just in case
                        Duration::from_millis(1000 / 60 + 1),
                        move || {
                            let s = weak.upgrade();
                            if let Some(s) = s {
                                s.on_first_frame();
                            }
                        },
                    )
                    .detach();
            }
        }
    }

    fn schedule_first_frame_notification(&self) {
        let weak = self.weak_self.borrow().clone();
        self.view.borrow().connect_draw(move |_, _| {
            let s = weak.upgrade();
            if let Some(s) = s {
                s.on_draw();
            }
            Inhibit(false)
        });
    }

    pub fn get_platform_window(&self) -> PlatformWindowType {
        self.window.clone()
    }

    pub fn show(&self) -> PlatformResult<()> {
        if self.ready_to_show.get() {
            self.window.show();
            Ok(())
        } else {
            self.show_when_ready.set(true);
            Ok(())
        }
    }

    pub fn hide(&self) -> PlatformResult<()> {
        if self.ready_to_show.get() {
            self.window.hide();
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.visibility_changed(false);
            }
        } else {
            self.show_when_ready.set(false);
        }
        Ok(())
    }

    pub fn activate(&self, _activate_application: bool) -> PlatformResult<bool> {
        self.window.present();
        Ok(true)
    }

    pub fn deactivate(&self, _deactivate_application: bool) -> PlatformResult<bool> {
        Ok(false)
    }

    pub fn ready_to_show(&self) -> PlatformResult<()> {
        self.ready_to_show.set(true);
        if self.show_when_ready.get() {
            self.window.show(); // otherwise complains about size allocation in show_all
            self.window.set_opacity(0.0);
            self.window.show_all();

            // The rest is done in on_first_frame
            Ok(())
        } else {
            Ok(())
        }
    }

    pub(super) fn on_event(&self, event: &mut Event) {
        if event.event_type() == EventType::ButtonPress
            || event.event_type() == EventType::ButtonRelease
            || event.event_type() == EventType::KeyPress
            || event.event_type() == EventType::KeyRelease
            || event.event_type() == EventType::MotionNotify
        {
            self.last_event
                .borrow_mut()
                .insert(event.event_type(), event.clone());
        }

        if event.event_type() == EventType::KeyPress {
            if let Some(context) = self.context.get() {
                context
                    .keyboard_map_manager
                    .borrow()
                    .borrow()
                    .platform_map
                    .on_key_event(event);
            }
        }

        if self.window_menu.borrow().should_forward_event(event) {
            self.propagate_event(event);
        }
    }

    pub(super) fn propagate_event(&self, event: &mut Event) {
        let event_box = self.get_event_box();
        if let Some(event_box) = event_box {
            let mut event = translate_event_to_window(event, &self.view.borrow().window().unwrap());
            propagate_event(&event_box, &mut event);
        }
    }

    pub fn close(&self) -> PlatformResult<()> {
        self.deleting.replace(true);
        self.window.close();
        Ok(())
    }

    pub fn close_with_result(&self, result: Value) -> PlatformResult<()> {
        let callback = self.modal_close_callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback(Ok(result));
        }
        self.close()
    }

    pub fn show_modal<F>(&self, done_callback: F)
    where
        F: FnOnce(PlatformResult<Value>) + 'static,
    {
        self.modal_close_callback
            .borrow_mut()
            .replace(Box::new(done_callback));

        if let Some(parent) = self.parent.as_ref().and_then(|p| p.upgrade()) {
            let parent_window = parent.window.clone();
            self.window.set_transient_for(Some(&parent_window));
        }

        self.window.set_modal(true);
        self.window
            .window()
            .unwrap()
            .set_type_hint(gdk::WindowTypeHint::Dialog);

        self.show().ok_log();
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        let geometry = &geometry.geometry;

        let request = self.last_geometry_request.borrow().update(GeometryRequest {
            frame_origin: geometry.frame_origin.clone(),
            content_size: geometry.content_size.clone(),
            min_content_size: geometry.min_content_size.clone(),
        });

        if self.window_size_in_progress.get() {
            self.pending_geometry_request.borrow_mut().replace(request);
        } else {
            self.pending_geometry_request.borrow_mut().take();
            let in_progress = self._set_geometry(request, false);
            self.window_size_in_progress.set(in_progress);
        }

        Ok(WindowGeometryFlags {
            frame_origin: geometry.frame_origin.is_some() && get_session_type() == SessionType::X11,
            content_size: geometry.content_size.is_some(),
            min_content_size: geometry.min_content_size.is_some(),
            ..Default::default()
        })
    }

    fn _set_geometry(&self, geometry: GeometryRequest, force: bool) -> bool {
        let moving = self.last_geometry_request.borrow().frame_origin != geometry.frame_origin;
        let resizing = self.last_geometry_request.borrow().content_size != geometry.content_size;

        if moving || force {
            if let Some(frame_origin) = &geometry.frame_origin {
                self.window
                    .move_(frame_origin.x as i32, frame_origin.y as i32);
            }
        }

        if resizing || force {
            if let Some(content_size) = &geometry.content_size {
                if !self.window.is_resizable() {
                    size_widget_set_min_size(
                        &self.size_widget,
                        content_size.width as i32,
                        content_size.height as i32,
                    );
                    self.window.queue_resize();
                } else {
                    self.window
                        .resize(content_size.width as i32, content_size.height as i32);
                }
            }
        }

        if self.window.is_resizable() {
            if let Some(min_content_size) = &geometry.min_content_size {
                size_widget_set_min_size(
                    &self.size_widget,
                    min_content_size.width as i32,
                    min_content_size.height as i32,
                );
            }
        }

        *self.last_geometry_request.borrow_mut() = geometry;

        resizing
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        let last_request = self.last_geometry_request.borrow();

        let frame_origin = if get_session_type() == SessionType::X11 {
            let origin = self.window.position();
            Some(Point::xy(origin.0 as f64, origin.1 as f64))
        } else {
            None
        };

        let content_size = self.window.size();
        let content_size = Size::wh(content_size.0 as f64, content_size.1 as f64);

        Ok(WindowGeometry {
            frame_origin,
            frame_size: None,
            content_origin: None,
            content_size: Some(content_size),
            min_frame_size: None,
            max_frame_size: None,
            min_content_size: last_request.min_content_size.clone(),
            max_content_size: None,
        })
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        Ok(WindowGeometryFlags {
            frame_origin: get_session_type() == SessionType::X11,
            content_size: true,
            min_content_size: true,
            ..Default::default()
        })
    }

    pub fn set_title(&self, title: String) -> PlatformResult<()> {
        self.window.set_title(&title);
        Ok(())
    }

    pub fn set_collection_behavior(
        &self,
        _behavior: WindowCollectionBehavior,
    ) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn set_minimized(&self, minimized: bool) -> PlatformResult<()> {
        let is_minimized = self.window_state.borrow().is_minimized;
        if minimized && !is_minimized {
            self.window.iconify();
        } else if !minimized && is_minimized {
            self.window.deiconify();
        }
        Ok(())
    }

    pub fn set_maximized(&self, maximized: bool) -> PlatformResult<()> {
        let is_maximized = self.window_state.borrow().is_maximized;
        if maximized && !is_maximized {
            self.window.maximize();
        } else if !maximized && is_maximized {
            self.window.unmaximize();
        }
        Ok(())
    }

    pub fn set_full_screen(&self, full_screen: bool) -> PlatformResult<()> {
        let is_full_screen = self.window_state.borrow().is_full_screen;
        if full_screen && !is_full_screen {
            self.window.fullscreen();
        } else if !full_screen && is_full_screen {
            self.window.unfullscreen();
        }
        Ok(())
    }

    pub fn get_screen_id(&self) -> PlatformResult<i64> {
        if let Some(display) = Display::default() {
            let monitor = display.monitor_at_window(&self.window.window().unwrap());
            Ok(monitor
                .as_ref()
                .map(PlatformScreenManager::get_monitor_id)
                .unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    pub fn get_window_state_flags(&self) -> PlatformResult<WindowStateFlags> {
        let state = self.window_state.borrow();
        Ok(WindowStateFlags {
            maximized: if state.is_maximized {
                BoolTransition::Yes
            } else {
                BoolTransition::No
            },
            minimized: if state.is_minimized {
                BoolTransition::Yes
            } else {
                BoolTransition::No
            },
            full_screen: if state.is_full_screen {
                BoolTransition::Yes
            } else {
                BoolTransition::No
            },
            active: state.is_active,
        })
    }

    pub fn save_position_to_string(&self) -> PlatformResult<String> {
        let state = self.window_state.borrow();
        Ok(serde_json::to_string(&*state).unwrap())
    }

    pub fn restore_position_from_string(&self, position: String) -> PlatformResult<()> {
        let state: WindowState =
            serde_json::from_str(&position).map_err(|_| PlatformError::OtherError {
                error: "Invalid window position string".into(),
            })?;

        self.window.resize(state.width, state.height);
        if state.is_maximized {
            self.window.maximize();
        }
        if state.is_full_screen {
            self.window.fullscreen();
        }

        Ok(())
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        self.last_window_style.borrow_mut().replace(style.clone());

        self.window.realize();

        let window = self.window.window().unwrap();
        match style.frame {
            WindowFrame::Regular => {
                window.set_decorations(WMDecoration::ALL);
            }
            WindowFrame::NoTitle => {
                window.set_decorations(WMDecoration::BORDER);
            }
            WindowFrame::NoFrame => {
                window.set_decorations(WMDecoration::empty());
            }
        }

        let prev_resizable = self.window.is_resizable();
        self.window.set_resizable(style.can_resize);

        let last_request = self.last_geometry_request.borrow().clone();
        if prev_resizable != style.can_resize && last_request.content_size.is_some() {
            // content size is set differently for resizable / non-resizable windows
            self._set_geometry(last_request, true);
        }

        let mut func = WMFunction::MOVE;
        if style.can_resize {
            func |= WMFunction::RESIZE;
        }
        if style.can_minimize {
            func |= WMFunction::MINIMIZE;
        }
        if style.can_maximize {
            func |= WMFunction::MAXIMIZE;
        }
        if style.can_close {
            func |= WMFunction::CLOSE;
        }

        window.set_functions(func);

        window.set_keep_above(style.always_on_top);

        Ok(())
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        if let Some(event) = self.last_event.borrow().get(&EventType::ButtonPress) {
            if let (Some(coords), Some(button)) = (event.root_coords(), event.button()) {
                // release event will get eaten, we need to synthetize it otherwise flutter keeps waiting for it
                let mut release = synthetize_button_up(event);
                gtk::main_do_event(&mut release);

                self.window.window().unwrap().begin_move_drag(
                    button as i32,
                    coords.0 as i32,
                    coords.1 as i32,
                    event.time(),
                );
            }
        }

        Ok(())
    }

    pub fn begin_drag_session(&self, request: DragRequest) -> PlatformResult<()> {
        // relase event will get eaten
        if let Some(event) = self.last_event.borrow().get(&EventType::ButtonPress) {
            let mut release = synthetize_button_up(event);
            gtk::main_do_event(&mut release);
        }

        self.drag_context
            .borrow()
            .begin_drag(request, self.get_event_box().as_ref().unwrap());
        Ok(())
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {
        self.drop_context.borrow().set_pending_effect(effect);
    }

    pub fn show_popup_menu<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<PopupMenuResponse>) + 'static,
    {
        self.window_menu
            .borrow()
            .show_popup_menu(menu, request, on_done)
    }

    pub fn hide_popup_menu(&self, menu: Rc<PlatformMenu>) -> PlatformResult<()> {
        self.window_menu.borrow().hide_popup_menu(menu)
    }

    pub fn show_system_menu(&self) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn set_window_menu(&self, _menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }
}

#[derive(Debug, Default, Clone)]
struct GeometryRequest {
    pub frame_origin: Option<Point>,
    pub content_size: Option<Size>,
    pub min_content_size: Option<Size>,
}

impl GeometryRequest {
    fn update(&self, req: GeometryRequest) -> Self {
        GeometryRequest {
            frame_origin: req.frame_origin.or_else(|| self.frame_origin.clone()),
            content_size: req.content_size.or_else(|| self.content_size.clone()),
            min_content_size: req
                .min_content_size
                .or_else(|| self.min_content_size.clone()),
        }
    }
}
