use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    shell::{
        api_model::{DragData, DragEffect, DragRequest, DraggingInfo},
        Context, ContextRef, IPoint,
    },
    util::{LateRefCell, OkLog},
};

use super::{
    drag_com::{DataObject, DropSource, DropTarget, DropTargetDelegate},
    drag_data::{
        DragDataAdapter, FallThroughDragDataAdapter, FilesDragDataAdapter, UrlsDragDataAdapter,
    },
    drag_util::{
        convert_drag_effect, convert_drag_effects, convert_drop_effect_mask,
        create_dragimage_bitmap, CLSID_DragDropHelper,
    },
    error::PlatformResult,
    util::create_instance,
    window::PlatformWindow,
};

use super::all_bindings::*;

pub struct DragContext {
    context: Context,
    weak_self: LateRefCell<Weak<DragContext>>,
    window: Weak<PlatformWindow>,
    drag_data: RefCell<Option<DragData>>,
    next_drag_effect: RefCell<DragEffect>,
    data_adapters: Vec<Box<dyn DragDataAdapter>>,
}

impl DragContext {
    pub fn new(context: &ContextRef, window: Weak<PlatformWindow>) -> Self {
        Self {
            context: context.weak(),
            weak_self: LateRefCell::new(),
            window,
            drag_data: RefCell::new(None),
            next_drag_effect: RefCell::new(DragEffect::None),
            data_adapters: vec![
                Box::new(FilesDragDataAdapter::new()),
                Box::new(UrlsDragDataAdapter::new()),
                Box::new(FallThroughDragDataAdapter::new(&context.options)),
            ],
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<DragContext>) {
        self.weak_self.set(weak_self);
        let window = self.window.upgrade().unwrap();
        let target: IDropTarget =
            DropTarget::new(window.hwnd(), self.weak_self.clone_value()).into();
        unsafe {
            RegisterDragDrop(window.hwnd(), target).ok_log();
        }
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {
        self.next_drag_effect.replace(effect);
    }

    pub fn shut_down(&self) -> PlatformResult<()> {
        let window = self.window.upgrade().unwrap();
        unsafe { RevokeDragDrop(window.hwnd()).map_err(|e| e.into()) }
    }

    pub fn begin_drag_session(&self, request: DragRequest) -> PlatformResult<()> {
        let weak = self.weak_self.clone_value();
        if let Some(context) = self.context.get() {
            context
                .run_loop
                .borrow()
                .schedule_now(move || {
                    if let Some(s) = weak.upgrade() {
                        unsafe {
                            s.start_drag_internal(request);
                        }
                    }
                })
                .detach();
        }
        Ok(())
    }

    fn serialize_drag_data(&self, mut data: DragData) -> HashMap<u32, Vec<u8>> {
        let mut res = HashMap::new();
        if let Some(context) = self.context.get() {
            for adapter in &context.options.custom_drag_data_adapters {
                adapter.prepare_drag_data(&mut data.properties, &mut res);
            }
        }
        for adapter in &self.data_adapters {
            adapter.prepare_drag_data(&mut data.properties, &mut res);
        }
        res
    }

    fn deserialize_drag_data(&self, data: IDataObject) -> DragData {
        let mut res: DragData = Default::default();

        if let Some(context) = self.context.get() {
            for adapter in &context.options.custom_drag_data_adapters {
                adapter.retrieve_drag_data(data.clone(), &mut res.properties);
            }
        }

        for adapter in &self.data_adapters {
            adapter.retrieve_drag_data(data.clone(), &mut res.properties);
        }

        res
    }

    unsafe fn start_drag_internal(&self, request: DragRequest) {
        let window = self.window.upgrade().unwrap();
        let data = self.serialize_drag_data(request.data);
        let data = Rc::new(RefCell::new(data));
        let data: IDataObject = DataObject::new(Rc::downgrade(&data)).into();
        let helper: IDragSourceHelper = create_instance(&CLSID_DragDropHelper).unwrap();
        let hbitmap = create_dragimage_bitmap(&request.image);
        let image_start = window.local_to_global(request.rect.origin());
        let mut cursor_pos = POINT::default();
        GetCursorPos(&mut cursor_pos as *mut _);

        let mut image = SHDRAGIMAGE {
            sizeDragImage: SIZE {
                cx: request.image.width,
                cy: request.image.height,
            },
            ptOffset: POINT {
                x: cursor_pos.x - image_start.x,
                y: cursor_pos.y - image_start.y,
            },
            hbmpDragImage: hbitmap,
            crColorKey: 0xFFFFFFFF,
        };
        helper
            .InitializeFromBitmap(&mut image as *mut _, data.clone())
            .ok_log();
        let source: IDropSource = DropSource::new().into();
        let ok_effects = convert_drag_effects(&request.allowed_effects);
        let mut effects_out: u32 = 0;
        let res = DoDragDrop(data, source, ok_effects, &mut effects_out as *mut u32);

        if let Some(delegate) = window.delegate() {
            let mut effect = DragEffect::None;
            if res == DRAGDROP_S_DROP {
                effect = convert_drop_effect_mask(effects_out)
                    .first()
                    .cloned()
                    .unwrap_or(DragEffect::None);
            }
            delegate.drag_ended(effect);
        }
    }
}

impl DropTargetDelegate for DragContext {
    fn drag_enter(&self, object: IDataObject, pt: &POINTL, effect_mask: u32) -> u32 {
        let window = self.window.upgrade().unwrap();
        if !window.is_enabled() {
            return DROPEFFECT_NONE;
        }
        let data = self.deserialize_drag_data(object);
        self.drag_data.replace(Some(data.clone()));
        self.next_drag_effect.replace(DragEffect::None);
        let pt = window.global_to_local(&IPoint::xy(pt.x, pt.y));
        let info = DraggingInfo {
            location: pt,
            data,
            allowed_effects: convert_drop_effect_mask(effect_mask),
        };
        if let Some(delegate) = window.delegate() {
            delegate.dragging_updated(&info);
        }
        convert_drag_effect(&self.next_drag_effect.borrow())
    }

    fn drag_over(&self, pt: &POINTL, effect_mask: u32) -> u32 {
        let window = self.window.upgrade().unwrap();
        if !window.is_enabled() {
            return DROPEFFECT_NONE;
        }
        let pt = window.global_to_local(&IPoint::xy(pt.x, pt.y));
        let info = DraggingInfo {
            location: pt,
            data: self.drag_data.borrow().clone().unwrap(),
            allowed_effects: convert_drop_effect_mask(effect_mask),
        };
        if let Some(delegate) = window.delegate() {
            delegate.dragging_updated(&info);
        }
        convert_drag_effect(&self.next_drag_effect.borrow())
    }

    fn drag_leave(&self) {
        let window = self.window.upgrade().unwrap();
        self.drag_data.replace(None);
        self.next_drag_effect.replace(DragEffect::None);
        if let Some(delegate) = window.delegate() {
            delegate.dragging_exited();
        }
    }

    fn perform_drop(&self, object: IDataObject, pt: &POINTL, effect_mask: u32) -> u32 {
        let window = self.window.upgrade().unwrap();
        if !window.is_enabled() {
            return DROPEFFECT_NONE;
        }
        let res = convert_drag_effect(&self.next_drag_effect.replace(DragEffect::None));
        let pt = window.global_to_local(&IPoint::xy(pt.x, pt.y));
        self.drag_data.replace(None);
        let info = DraggingInfo {
            location: pt,
            data: self.deserialize_drag_data(object),
            allowed_effects: convert_drop_effect_mask(effect_mask),
        };
        if let Some(delegate) = window.delegate() {
            delegate.perform_drop(&info);
        }
        res
    }
}
