use std::{
    cell::Cell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::shell::{
    api_model::{DragData, DragEffect, DragRequest, DraggingInfo},
    Context, ContextRef, PlatformWindowDelegate, Point,
};
use cocoa::{
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSInteger, NSPoint, NSRect, NSUInteger},
};
use objc::{
    class, msg_send,
    rc::{autoreleasepool, StrongPtr},
    sel, sel_impl,
};

use super::{
    drag_data::{
        DragDataAdapter, FallThroughDragDataAdapter, FilesDragDataAdapter, PasteboardItems,
        UrlsDragDataAdapter,
    },
    utils::{array_with_objects, flip_rect, ns_image_from},
    window::PlatformWindow,
};

pub type NSDragOperation = NSUInteger;

pub struct DragContext {
    context: Context,
    window: Weak<PlatformWindow>,
    next_drag_operation: Cell<NSDragOperation>,
    data_adapters: Vec<Box<dyn DragDataAdapter>>,
    allowed_operations: Cell<NSDragOperation>,
}

#[allow(non_upper_case_globals)]
const NSDragOperationNone: NSDragOperation = 0;
#[allow(non_upper_case_globals)]
const NSDragOperationCopy: NSDragOperation = 1;
#[allow(non_upper_case_globals)]
const NSDragOperationLink: NSDragOperation = 2;
#[allow(non_upper_case_globals)]
const NSDragOperationMove: NSDragOperation = 16;

impl DragContext {
    pub fn new(context: &ContextRef, window: Weak<PlatformWindow>) -> Self {
        Self {
            context: context.weak(),
            window,
            next_drag_operation: Cell::new(NSDragOperationNone),
            data_adapters: vec![
                Box::new(FilesDragDataAdapter::new()),
                Box::new(UrlsDragDataAdapter::new()),
                Box::new(FallThroughDragDataAdapter::new(&context.options)),
            ],
            allowed_operations: Cell::new(NSDragOperationNone),
        }
    }

    pub fn register(&self, window: id) {
        if let Some(context) = self.context.get() {
            let mut types = Vec::<StrongPtr>::new();

            for adapter in &context.options.custom_drag_data_adapters {
                adapter.register_types(&mut types);
            }
            for adapter in &self.data_adapters {
                adapter.register_types(&mut types);
            }

            unsafe {
                let types = array_with_objects(&types);
                let () = msg_send![window, registerForDraggedTypes: types];
            }
        }
    }

    pub fn set_pending_effect(&self, effect: DragEffect) {
        self.next_drag_operation.set(convert_drag_effect(&effect));
    }

    fn with_delegate<F>(&self, callback: F)
    where
        F: FnOnce(Rc<dyn PlatformWindowDelegate>),
    {
        let win = self.window.upgrade();
        if let Some(win) = win {
            win.with_delegate(callback);
        }
    }

    pub fn dragging_entered(&self, dragging_info: id) -> NSDragOperation {
        self.dragging_updated(dragging_info);
        NSDragOperationNone
    }

    pub fn dragging_updated(&self, dragging_info: id) -> NSDragOperation {
        let info = self.convert_dragging_info(dragging_info);
        self.with_delegate(|delegate| {
            delegate.dragging_updated(&info);
        });

        self.next_drag_operation.get()
    }

    pub fn dragging_exited(&self, _dragging_info: id) {
        if let Some(window) = self.window.upgrade() {
            window.synthetize_mouse_move_if_needed();
        }
        self.with_delegate(|delegate| {
            delegate.dragging_exited();
        });
    }

    pub fn perform_drag_operation(&self, dragging_info: id) -> BOOL {
        let info = self.convert_dragging_info(dragging_info);
        if let Some(window) = self.window.upgrade() {
            window.synthetize_mouse_move_if_needed();
        }
        self.with_delegate(|delegate| {
            delegate.perform_drop(&info);
        });
        if self.next_drag_operation.get() != NSDragOperationNone {
            YES
        } else {
            NO
        }
    }

    pub fn convert_dragging_info(&self, dragging_info: id) -> DraggingInfo {
        autoreleasepool(|| unsafe {
            let window = self.window.upgrade().unwrap();
            let location: NSPoint = msg_send![dragging_info, draggingLocation];
            let location = Point::xy(location.x, window.get_content_size().height - location.y);
            let pasteboard: id = msg_send![dragging_info, draggingPasteboard];

            let mut data = HashMap::new();
            if let Some(context) = self.context.get() {
                for adapter in &context.options.custom_drag_data_adapters {
                    adapter.retrieve_drag_data(pasteboard, &mut data);
                }
            }
            for adapter in &self.data_adapters {
                adapter.retrieve_drag_data(pasteboard, &mut data);
            }
            let data = DragData { properties: data };

            let operation_mask: NSDragOperation =
                msg_send![dragging_info, draggingSourceOperationMask];
            let allowed_effects = convert_operation_mask(operation_mask);

            DraggingInfo {
                location,
                data,
                allowed_effects,
            }
        })
    }

    pub unsafe fn start_drag(&self, request: DragRequest, view: id, source: id, event: id) {
        let mut pasteboard_items = PasteboardItems::new();

        let mut data = request.data.properties;

        if let Some(context) = self.context.get() {
            for adapter in &context.options.custom_drag_data_adapters {
                pasteboard_items.reset_index();
                adapter.prepare_drag_data(&mut data, &mut pasteboard_items);
            }
        }
        for adapter in &self.data_adapters {
            pasteboard_items.reset_index();
            adapter.prepare_drag_data(&mut data, &mut pasteboard_items);
        }

        let mut first = true;
        let mut dragging_items = Vec::<StrongPtr>::new();
        let snapshot = ns_image_from(vec![request.image]);
        let mut rect: NSRect = request.rect.into();
        flip_rect(view, &mut rect);

        for item in pasteboard_items.get_items() {
            let dragging_item: id = msg_send![class!(NSDraggingItem), alloc];
            let dragging_item =
                StrongPtr::new(msg_send![dragging_item, initWithPasteboardWriter:*item]);
            let () = msg_send![*dragging_item,
               setDraggingFrame:rect
               contents:if first {*snapshot } else {nil}
            ];
            first = false;

            dragging_items.push(dragging_item);
        }

        let mut allowed_operations = 0;

        for e in request.allowed_effects {
            allowed_operations |= convert_drag_effect(&e);
        }

        self.allowed_operations.replace(allowed_operations);

        let _session: id = msg_send![view,
            beginDraggingSessionWithItems:array_with_objects(&dragging_items)
            event:event
            source:source
        ];
    }

    pub fn source_operation_mask_for_dragging_context(
        &self,
        _session: id,
        _context: NSInteger,
    ) -> NSDragOperation {
        self.allowed_operations.get()
    }

    pub fn drag_ended(&self, _session: id, _point: NSPoint, operation: NSDragOperation) {
        #[allow(non_upper_case_globals)]
        let effect = match operation {
            NSDragOperationCopy => DragEffect::Copy,
            NSDragOperationLink => DragEffect::Link,
            NSDragOperationMove => DragEffect::Move,
            _ => DragEffect::None,
        };
        if let Some(window) = self.window.upgrade() {
            window.synthetize_mouse_move_if_needed();
        }
        self.with_delegate(|delegate| {
            delegate.drag_ended(effect);
        });
    }
}

fn convert_drag_effect(effect: &DragEffect) -> NSDragOperation {
    match effect {
        DragEffect::None => NSDragOperationNone,
        DragEffect::Copy => NSDragOperationCopy,
        DragEffect::Link => NSDragOperationLink,
        DragEffect::Move => NSDragOperationMove,
    }
}

fn convert_operation_mask(operation_mask: NSDragOperation) -> Vec<DragEffect> {
    let mut res = Vec::new();
    if operation_mask & NSDragOperationCopy == NSDragOperationCopy {
        res.push(DragEffect::Copy);
    }
    if operation_mask & NSDragOperationLink == NSDragOperationLink {
        res.push(DragEffect::Link);
    }
    if operation_mask & NSDragOperationMove == NSDragOperationMove {
        res.push(DragEffect::Move);
    }
    res
}
