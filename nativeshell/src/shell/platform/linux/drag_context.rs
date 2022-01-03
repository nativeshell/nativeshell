use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::take,
    rc::{Rc, Weak},
};

use cairo::{Format, ImageSurface};
use gdk::{Atom, DragAction, EventType};
use glib::IsA;
use gtk::{
    prelude::{DragContextExtManual, WidgetExt, WidgetExtManual},
    DestDefaults, SelectionData, TargetEntry, TargetFlags, TargetList, Widget,
};

use crate::{
    codec::Value,
    shell::{
        api_model::{DragData, DragEffect, DragRequest, DraggingInfo, ImageData},
        platform::drag_data::{FallThroughDragDataAdapter, UriListDataAdapter},
        Context, ContextRef, PlatformWindowDelegate, Point,
    },
};

use super::{
    drag_data::{DragDataAdapter, DragDataSetter},
    window::PlatformWindow,
};

pub struct DropContext {
    context: Context,
    window: Weak<PlatformWindow>,
    data_adapters: Vec<Box<dyn DragDataAdapter>>,

    current_data: RefCell<HashMap<String, Value>>,
    pending_data: RefCell<Vec<Atom>>,
    drag_location: RefCell<Point>,
    pending_effect: Cell<DragEffect>,
    drag_context: RefCell<Option<gdk::DragContext>>,
    dropping: Cell<bool>,
}

impl DropContext {
    pub fn new(context: &ContextRef, window: Weak<PlatformWindow>) -> Self {
        Self {
            context: context.weak(),
            window,
            data_adapters: vec![
                Box::new(UriListDataAdapter::new()),
                Box::new(FallThroughDragDataAdapter::new(&context.options)),
            ],
            current_data: RefCell::new(HashMap::new()),
            pending_data: RefCell::new(Vec::new()),
            drag_location: RefCell::new(Default::default()),
            pending_effect: Cell::new(DragEffect::None),
            drag_context: RefCell::new(None),
            dropping: Cell::new(false),
        }
    }

    fn data_adapters<'a>(&'a self, context: &'a ContextRef) -> Vec<&'a dyn DragDataAdapter> {
        context
            .options
            .custom_drag_data_adapters
            .iter()
            .chain(self.data_adapters.iter())
            .map(|a| a.as_ref())
            .collect()
    }

    pub fn drag_motion<T: IsA<Widget>>(
        &self,
        widget: &T,
        context: &gdk::DragContext,
        x: i32,
        y: i32,
        time: u32,
    ) {
        *self.drag_location.borrow_mut() = Point::xy(x as f64, y as f64);
        self.drag_context.borrow_mut().replace(context.clone());
        self.dropping.replace(false);

        if !self.pending_data.borrow().is_empty() {
            return;
        }

        self.get_data(widget, context, time);
    }

    fn get_data<T: IsA<Widget>>(&self, widget: &T, context: &gdk::DragContext, time: u32) {
        let pending_data = {
            let mut pending_data = self.pending_data.borrow_mut();

            if !pending_data.is_empty() {
                return;
            }

            if let Some(ctx) = self.context.get() {
                let mut adapters = self.data_adapters(&ctx);

                for target in context.list_targets() {
                    let adapter_index = adapters
                        .iter()
                        .position(|p| p.data_formats().contains(&target));
                    if let Some(adapter_index) = adapter_index {
                        pending_data.push(target);
                        adapters.remove(adapter_index);
                    }
                }
            }

            pending_data.clone()
        };

        for data in pending_data.iter() {
            widget.drag_get_data(context, data, time);
        }
    }

    fn cleanup(&self) {
        self.current_data.borrow_mut().clear();
        self.pending_data.borrow_mut().clear();
        self.pending_effect.replace(DragEffect::None);
        self.drag_context.borrow_mut().take();
        self.dropping.replace(false);
    }

    pub fn drag_leave<T: IsA<Widget>>(&self, _widget: &T, _context: &gdk::DragContext, _time: u32) {
        self.cleanup();

        self.with_delegate(|d| d.dragging_exited());
    }

    pub fn drag_drop<T: IsA<Widget>>(
        &self,
        widget: &T,
        context: &gdk::DragContext,
        x: i32,
        y: i32,
        time: u32,
    ) {
        *self.drag_location.borrow_mut() = Point::xy(x as f64, y as f64);
        self.dropping.replace(true);

        if self.pending_data.borrow().is_empty() {
            self.get_data(widget, context, time);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn drag_data_received<T: IsA<Widget>>(
        &self,
        _widget: &T,
        context: &gdk::DragContext,
        _x: i32, // always zero
        _y: i32, // always zero
        data: &SelectionData,
        _info: u32,
        time: u32,
    ) {
        {
            let mut pending_data = self.pending_data.borrow_mut();
            if pending_data.is_empty() {
                return;
            }

            if let Some(ctx) = self.context.get() {
                let adapters = self.data_adapters(&ctx);
                let data_type = data.data_type();
                if let Some(pos) = pending_data.iter().position(|d| d == &data_type) {
                    pending_data.remove(pos);
                    for adapter in adapters {
                        if adapter.data_formats().contains(&data_type) {
                            adapter.retrieve_drag_data(data, &mut self.current_data.borrow_mut());
                        }
                    }
                }
            }
        }

        if self.pending_data.borrow().is_empty() {
            // We're done here
            let info = DraggingInfo {
                location: self.drag_location.borrow().clone(),
                data: DragData {
                    properties: take(&mut self.current_data.borrow_mut()),
                },
                allowed_effects: Self::convert_drag_actions_from_gtk(context.actions()),
            };
            if !self.dropping.get() {
                self.with_delegate(|d| d.dragging_updated(&info));
            } else {
                self.with_delegate(|d| d.perform_drop(&info));
                context.drag_finish(true, self.pending_effect.get() == DragEffect::Move, time);
                self.cleanup();
            }
        }
    }

    fn convert_drag_actions_from_gtk(actions: DragAction) -> Vec<DragEffect> {
        let mut res = Vec::new();
        if actions.contains(DragAction::MOVE) {
            res.push(DragEffect::Move);
        }
        if actions.contains(DragAction::COPY) {
            res.push(DragEffect::Copy);
        }
        if actions.contains(DragAction::LINK) {
            res.push(DragEffect::Link);
        }
        res
    }

    fn convert_effect_to_gtk(effect: DragEffect) -> DragAction {
        match effect {
            DragEffect::None => DragAction::empty(),
            DragEffect::Copy => DragAction::COPY,
            DragEffect::Link => DragAction::LINK,
            DragEffect::Move => DragAction::MOVE,
        }
    }

    fn with_delegate<F>(&self, callback: F)
    where
        F: FnOnce(Rc<dyn PlatformWindowDelegate>),
    {
        let win = self.window.upgrade();
        if let Some(delegate) = win.and_then(|w| w.delegate.upgrade()) {
            callback(delegate);
        }
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {
        self.pending_effect.replace(effect);
        if let Some(drag_context) = self.drag_context.borrow().clone() {
            drag_context.drag_status(Self::convert_effect_to_gtk(effect), 0);
        }
    }

    pub fn register<T: IsA<Widget>>(&self, widget: &T) {
        let mut atoms = Vec::<Atom>::new();
        if let Some(ctx) = self.context.get() {
            let adapters = self.data_adapters(&ctx);

            for adapter in adapters {
                adapter.data_formats().iter().for_each(|a| atoms.push(*a));
            }
        }
        let entries: Vec<TargetEntry> = atoms
            .iter()
            .map(|a| TargetEntry::new(&a.name(), TargetFlags::empty(), 0))
            .collect();
        widget.drag_dest_set(
            // Gtk documentation says that when calling get_drag_data from drag_motion the
            // DestDefaults::DROP flag should be set, but that causes nautilus to lock up.
            // Not having the flag and calling drag_finish manually seems to work fine
            DestDefaults::empty(),
            &entries,
            DragAction::MOVE | DragAction::COPY | DragAction::LINK,
        );
    }
}

//
//
//

pub struct DragContext {
    context: Context,
    window: Weak<PlatformWindow>,
    data_adapters: Vec<Box<dyn DragDataAdapter>>,
    data: RefCell<Vec<Box<dyn DragDataSetter>>>,
    dragging: Cell<bool>,
}

impl DragContext {
    pub fn new(context: &ContextRef, window: Weak<PlatformWindow>) -> Self {
        Self {
            context: context.weak(),
            window,
            data_adapters: vec![
                Box::new(UriListDataAdapter::new()),
                Box::new(FallThroughDragDataAdapter::new(&context.options)),
            ],
            data: RefCell::new(Vec::new()),
            dragging: Cell::new(false),
        }
    }

    fn prepare_data(&self, request: &mut DragRequest) -> TargetList {
        let properties = &mut request.data.properties;
        let mut data = self.data.borrow_mut();
        data.clear();

        if let Some(context) = self.context.get() {
            for a in &context.options.custom_drag_data_adapters {
                data.append(&mut a.prepare_drag_data(properties));
            }
        }
        for a in &self.data_adapters {
            data.append(&mut a.prepare_drag_data(properties));
        }

        let targets = TargetList::new(&[]);
        data.iter().enumerate().all(|(index, source)| {
            for k in source.data_formats() {
                targets.add(&k, 0, index as u32);
            }
            true
        });

        targets
    }

    fn convert_effects_to_gtk(effects: &[DragEffect]) -> DragAction {
        let mut res = DragAction::empty();
        for e in effects {
            res |= DropContext::convert_effect_to_gtk(*e);
        }
        res
    }

    pub fn begin_drag<T: IsA<Widget>>(&self, mut request: DragRequest, widget: &T) {
        let window = self.window.upgrade().unwrap();

        let events = window.last_event.borrow();
        let drag_event = events
            .values()
            .filter(|e| {
                e.event_type() == EventType::ButtonPress
                    || e.event_type() == EventType::MotionNotify
            })
            .max_by(|e1, e2| e1.time().cmp(&e2.time()));

        let button = events
            .get(&EventType::ButtonPress)
            .and_then(|e| e.button())
            .unwrap_or(0);

        let targets = self.prepare_data(&mut request);

        let context = widget.drag_begin_with_coordinates(
            &targets,
            Self::convert_effects_to_gtk(&request.allowed_effects),
            button as i32,
            drag_event,
            -1,
            -1,
        );

        let event_coords = drag_event.and_then(|e| e.coords()).unwrap_or((0.0, 0.0));

        if let Some(context) = context {
            let surface = &Self::surface_from_image_data(request.image);
            let scale_factor = widget.scale_factor() as f64;
            surface.set_device_scale(widget.scale_factor() as f64, widget.scale_factor() as f64);
            surface.set_device_offset(
                (request.rect.x - event_coords.0) * scale_factor,
                (request.rect.y - event_coords.1) * scale_factor,
            );
            context.drag_set_icon_surface(surface)
        }

        self.dragging.replace(true);
    }

    fn surface_from_image_data(image: ImageData) -> ImageSurface {
        let mut data = image.data;
        for offset in (0..data.len()).step_by(4) {
            let (r, g, b, a) = (
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            );
            data[offset] = b;
            data[offset + 1] = g;
            data[offset + 2] = r;
            data[offset + 3] = a;
        }
        let surface = ImageSurface::create_for_data(
            data,
            Format::ARgb32,
            image.width,
            image.height,
            image.bytes_per_row,
        );
        surface.unwrap()
    }

    pub fn get_data(&self, selection_data: &SelectionData, target_info: u32) {
        let data = self.data.borrow();
        let data = data.get(target_info as usize);
        if let Some(data) = data {
            data.set(selection_data);
        }
    }

    fn with_delegate<F>(&self, callback: F)
    where
        F: FnOnce(Rc<dyn PlatformWindowDelegate>),
    {
        let win = self.window.upgrade();
        if let Some(delegate) = win.and_then(|w| w.delegate.upgrade()) {
            callback(delegate);
        }
    }

    fn cleanup(&self) {
        self.data.borrow_mut().clear();
        self.dragging.replace(false);
    }

    pub fn drag_failed(&self) {
        self.cleanup();
        self.with_delegate(|d| d.drag_ended(DragEffect::None));
    }

    pub fn drag_end(&self, context: &gdk::DragContext) {
        if self.dragging.get() {
            // if failes it means drag faile
            self.cleanup();
            let action = context.selected_action();
            let action = DropContext::convert_drag_actions_from_gtk(action)
                .first()
                .cloned()
                .unwrap_or(DragEffect::None);
            self.with_delegate(|d| d.drag_ended(action));
        }
    }
}
