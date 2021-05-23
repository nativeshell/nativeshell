use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    time::Duration,
};

use gdk::{Event, EventType, Geometry, Gravity, WMDecoration, WMFunction, WindowExt, WindowHints};
use glib::{Cast, ObjectExt, Type};
use gtk::{
    propagate_event, ContainerExt, EventBox, GtkWindowExt, Inhibit, Overlay, OverlayExt, Widget,
    WidgetExt,
};

use crate::{
    codec::Value,
    shell::{
        api_model::{
            DragEffect, DragRequest, PopupMenuRequest, PopupMenuResponse, WindowFrame,
            WindowGeometry, WindowGeometryFlags, WindowGeometryRequest, WindowStyle,
        },
        Context, IRect, ISize, PlatformWindowDelegate, Point, ScheduledCallback, Size,
    },
    util::{LateRefCell, OkLog},
};

use super::{
    drag_context::{DragContext, DropContext},
    engine::PlatformEngine,
    error::{PlatformError, PlatformResult},
    flutter::View,
    menu::PlatformMenu,
    utils::{get_session_type, synthetize_button_up, translate_event_to_window, SessionType},
    window_menu::WindowMenu,
};

pub type PlatformWindowType = gtk::Window;

struct Global {
    window_count: Cell<i32>,
}

unsafe impl Sync for Global {}

lazy_static! {
    static ref GLOBAL: Global = Global {
        window_count: Cell::new(0),
    };
}

pub struct PlatformWindow {
    context: Rc<Context>,
    pub(super) window: gtk::Window,
    weak_self: LateRefCell<Weak<PlatformWindow>>,
    parent: Option<Rc<PlatformWindow>>,
    pub(super) delegate: Weak<dyn PlatformWindowDelegate>,
    modal_close_callback: RefCell<Option<Box<dyn FnOnce(PlatformResult<Value>) -> ()>>>,
    pub(super) view: LateRefCell<View>,
    ready_to_show: Cell<bool>,
    show_when_ready: Cell<bool>,
    pending_first_frame: Cell<bool>,
    last_geometry_request: RefCell<Option<WindowGeometryRequest>>,
    last_window_style: RefCell<Option<WindowStyle>>,
    pub(super) last_event: RefCell<HashMap<EventType, Event>>,
    resize_finish_handle: RefCell<Option<ScheduledCallback>>,
    deleting: Cell<bool>,
    pub(super) window_menu: LateRefCell<WindowMenu>,
    pub(super) drop_context: LateRefCell<DropContext>,
    drag_context: LateRefCell<DragContext>,
}

impl PlatformWindow {
    pub fn new(
        context: Rc<Context>,
        delegate: Weak<dyn PlatformWindowDelegate>,
        parent: Option<Rc<PlatformWindow>>,
    ) -> Self {
        GLOBAL.window_count.replace(GLOBAL.window_count.get() + 1);

        Self {
            context,
            window: gtk::Window::new(gtk::WindowType::Toplevel),
            weak_self: LateRefCell::new(),
            delegate,
            modal_close_callback: RefCell::new(None),
            parent,
            view: LateRefCell::new(),
            ready_to_show: Cell::new(false),
            show_when_ready: Cell::new(false),
            pending_first_frame: Cell::new(true),
            last_geometry_request: RefCell::new(None),
            last_window_style: RefCell::new(None),
            last_event: RefCell::new(HashMap::new()),
            resize_finish_handle: RefCell::new(None),
            deleting: Cell::new(false),
            window_menu: LateRefCell::new(),
            drop_context: LateRefCell::new(),
            drag_context: LateRefCell::new(),
        }
    }

    pub fn on_first_frame(&self) {
        self.window.set_opacity(1.0);
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformWindow>, engine: &PlatformEngine) {
        self.weak_self.set(weak.clone());

        self.window_menu.set(WindowMenu::new(weak.clone()));

        let overlay = Overlay::new();
        self.window.add(&overlay);

        overlay.add(&self.window_menu.borrow().menu_bar_container);

        self.view.set(engine.view.clone());
        overlay.add_overlay(&self.view.borrow().clone());
        self.view.borrow().grab_focus();

        self.update_preferred_size();

        self.window.realize();
        unsafe {
            self.window
                .get_window()
                .unwrap()
                .set_data("nativeshell_platform_window", weak.clone());
        }

        // by default make window resizable, non resizable window need size
        // specified
        self.window.set_resizable(true);

        self.schedule_first_frame_notification();

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

        self.drop_context
            .set(DropContext::new(self.context.clone(), weak.clone()));
        self.drag_context
            .set(DragContext::new(self.context.clone(), weak.clone()));
        self.connect_drag_drop_events();
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
                Inhibit(true)
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
                Inhibit(true)
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

    // Override preferred width and height on FlView; We'll be taking care of window sizing
    // so just return minimal value required for Gtk to not use the default 200
    fn update_preferred_size(&self) {
        let gtype = self.view.borrow().get_type();
        if let Type::Other(gtype) = gtype {
            unsafe {
                let c = gobject_sys::g_type_class_peek(gtype) as *mut gtk_sys::GtkWidgetClass;
                let c = &mut *c;
                c.get_preferred_width = Some(Self::get_preferred_size);
                c.get_preferred_height = Some(Self::get_preferred_size);
            }
        }
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

    unsafe extern "C" fn get_preferred_size(
        _widget: *mut gtk_sys::GtkWidget,
        minimum: *mut i32,
        natural: *mut i32,
    ) {
        *minimum = 1;
        *natural = 1;
    }

    fn schedule_first_frame_notification(&self) {
        let weak = self.weak_self.borrow().clone();
        self.view.borrow().connect_draw(move |_, _| {
            let s = weak.upgrade();
            if let Some(s) = s {
                if s.pending_first_frame.get() {
                    s.view.borrow().forall(|w| {
                        // don't start until there is FlGLArea widget
                        if w.get_type().name() == "FlGLArea" {
                            s.pending_first_frame.replace(false);
                            let weak = weak.clone();
                            s.context
                                .run_loop
                                .borrow()
                                .schedule(
                                    move || {
                                        let s = weak.upgrade();
                                        if let Some(s) = s {
                                            s.on_first_frame();
                                        }
                                    },
                                    // delay one frame, just in case
                                    Duration::from_millis(1000 / 60 + 1),
                                )
                                .detach();
                        }
                    });
                }
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
        if event.get_event_type() == EventType::ButtonPress
            || event.get_event_type() == EventType::ButtonRelease
            || event.get_event_type() == EventType::KeyPress
            || event.get_event_type() == EventType::KeyRelease
            || event.get_event_type() == EventType::MotionNotify
        {
            self.last_event
                .borrow_mut()
                .insert(event.get_event_type(), event.clone());
        }

        if self.window_menu.borrow().should_forward_event(&event) {
            self.propagate_event(event);
        }
    }

    pub(super) fn propagate_event(&self, event: &mut Event) {
        let event_box = self.get_event_box();
        if let Some(event_box) = event_box {
            let mut event =
                translate_event_to_window(&event, &self.view.borrow().get_window().unwrap());
            propagate_event(&event_box, &mut event);
        }
    }

    // see init_platform for explanation
    pub(super) fn on_move_resize(&self, target_rect: IRect) {
        if let Some(style) = self.last_window_style.borrow().as_ref() {
            if !style.can_resize {
                // gtk on x11 needs tight constraints for window to be non resizable
                self.window.get_window().unwrap().set_geometry_hints(
                    Some(&Geometry {
                        min_width: target_rect.width,
                        min_height: target_rect.height,
                        max_width: target_rect.width,
                        max_height: target_rect.height,
                        base_width: 0,
                        base_height: 0,
                        width_inc: 0,
                        height_inc: 0,
                        min_aspect: 0.0,
                        max_aspect: 0.0,
                        win_gravity: Gravity::NorthWest,
                    })
                    .unwrap(),
                    WindowHints::MIN_SIZE | WindowHints::MAX_SIZE,
                );
            }
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
        F: FnOnce(PlatformResult<Value>) -> () + 'static,
    {
        self.modal_close_callback
            .borrow_mut()
            .replace(Box::new(done_callback));

        if let Some(parent) = self.parent.as_ref() {
            let parent_window = parent.window.clone();
            self.window.set_transient_for(Some(&parent_window));
        }

        self.window.set_modal(true);
        self.window
            .get_window()
            .unwrap()
            .set_type_hint(gdk::WindowTypeHint::Dialog);

        self.show().ok_log();
    }

    pub fn set_geometry(
        &self,
        geometry: WindowGeometryRequest,
    ) -> PlatformResult<WindowGeometryFlags> {
        self.last_geometry_request
            .borrow_mut()
            .replace(geometry.clone());

        let geometry = &geometry.geometry;

        // first set_geometry is always tight; between first and last set_geometry
        // during continuous resizing we want tight=false; that relaxes constaints
        // on windows, which results in smoother resizing

        self._set_geometry(geometry.clone());
        let weak = self.weak_self.borrow().clone();
        let geometry_clone = geometry.clone();

        // It is possible that set_geometry is invoked while window.resize is in progress;
        // that can happen because during synchronized resizing we are processing platform thread
        // tasks. If that's the case, some calls to window.resize might get lost in the process
        // so to make sure that doesn't happen schedule another call on main loop;
        let handle = self.context.run_loop.borrow().schedule(
            move || {
                let s = weak.upgrade();
                if let Some(s) = s {
                    s._set_geometry(geometry_clone);
                }
            },
            Duration::from_millis(1000 / 60 + 1),
        );
        self.resize_finish_handle.borrow_mut().replace(handle);

        Ok(WindowGeometryFlags {
            frame_origin: geometry.frame_origin.is_some() && get_session_type() == SessionType::X11,
            content_size: geometry.content_size.is_some(),
            min_content_size: geometry.min_content_size.is_some(),
            max_content_size: geometry.max_content_size.is_some(),
            ..Default::default()
        })
    }

    fn _set_geometry(&self, geometry: WindowGeometry) {
        if let Some(frame_origin) = &geometry.frame_origin {
            self.window
                .move_(frame_origin.x as i32, frame_origin.y as i32);
        }
        if let Some(content_size) = &geometry.content_size {
            self.window
                .resize(content_size.width as i32, content_size.height as i32);
        }

        let mut hints = WindowHints::empty();
        if geometry.min_content_size.is_some() || !self.window.get_resizable() {
            hints |= WindowHints::MIN_SIZE;
        }
        if geometry.max_content_size.is_some() || !self.window.get_resizable() {
            hints |= WindowHints::MAX_SIZE;
        }

        let min_content_size: ISize = geometry
            .min_content_size
            .clone()
            .unwrap_or(Size::wh(0.0, 0.0))
            .into();

        let max_content_size: ISize = geometry
            .min_content_size
            .clone()
            .unwrap_or(Size::wh(10000.0, 10000.0))
            .into();

        self.window.set_geometry_hints(
            None::<&Widget>,
            Some(&Geometry {
                min_width: min_content_size.width,
                min_height: min_content_size.height,
                max_width: max_content_size.width,
                max_height: max_content_size.height,
                base_width: 0,
                base_height: 0,
                width_inc: 0,
                height_inc: 0,
                min_aspect: 0.0,
                max_aspect: 0.0,
                win_gravity: Gravity::NorthWest,
            }),
            hints,
        );
    }

    pub fn get_geometry(&self) -> PlatformResult<WindowGeometry> {
        let last_request = self.last_geometry_request.borrow();
        let last_request = last_request
            .as_ref()
            .map(|r| r.geometry.clone())
            .unwrap_or_default();

        let frame_origin = if get_session_type() == SessionType::X11 {
            let origin = self.window.get_position();
            Some(Point::xy(origin.0 as f64, origin.1 as f64))
        } else {
            None
        };

        let content_size = self.window.get_size();
        let content_size = Size::wh(content_size.0 as f64, content_size.1 as f64);

        Ok(WindowGeometry {
            frame_origin: frame_origin,
            frame_size: None,
            content_origin: None,
            content_size: Some(content_size),
            min_frame_size: None,
            max_frame_size: None,
            min_content_size: last_request.min_content_size,
            max_content_size: last_request.max_content_size,
        })
    }

    pub fn supported_geometry(&self) -> PlatformResult<WindowGeometryFlags> {
        Ok(WindowGeometryFlags {
            frame_origin: get_session_type() == SessionType::X11,
            content_size: true,
            min_content_size: true,
            max_content_size: true,
            ..Default::default()
        })
    }

    pub fn set_style(&self, style: WindowStyle) -> PlatformResult<()> {
        self.last_window_style.borrow_mut().replace(style.clone());

        self.window.realize();

        let window = self.window.get_window().unwrap();
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

        self.window.set_resizable(style.can_resize);

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

        // update geometry - this is needed to set proper min/max size for (non)resizable windows
        let geometry = self.last_geometry_request.borrow().clone();
        if let Some(geometry) = geometry {
            self.set_geometry(geometry).ok_log();
        }

        Ok(())
    }

    pub fn perform_window_drag(&self) -> PlatformResult<()> {
        if let Some(event) = self.last_event.borrow().get(&EventType::ButtonPress) {
            if let (Some(coords), Some(button)) = (event.get_root_coords(), event.get_button()) {
                // release event will get eaten, we need to synthetize it otherwise flutter keeps waiting for it
                let mut release = synthetize_button_up(event);
                gtk::main_do_event(&mut release);

                self.window.get_window().unwrap().begin_move_drag(
                    button as i32,
                    coords.0 as i32,
                    coords.1 as i32,
                    event.get_time(),
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
        F: FnOnce(PlatformResult<PopupMenuResponse>) -> () + 'static,
    {
        self.window_menu
            .borrow()
            .show_popup_menu(menu, request, on_done)
    }

    pub fn hide_popup_menu(&self, menu: Rc<PlatformMenu>) -> PlatformResult<()> {
        self.window_menu.borrow().hide_popup_menu(menu)
    }

    pub fn show_system_menu(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    pub fn set_window_menu(&self, _menu: Option<Rc<PlatformMenu>>) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}
