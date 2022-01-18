use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodCallResult, Value,
    },
    util::{Late, OkLog},
    Error, Result,
};

use super::{
    api_constants::{channel, method},
    api_model::{
        StatusItemAction, StatusItemActionType, StatusItemCreateRequest, StatusItemDestroyRequest,
        StatusItemGetGeometryRequest, StatusItemGetScreenIdRequest,
        StatusItemSetHighlightedRequest, StatusItemSetHintRequest, StatusItemSetImageRequest,
        StatusItemShowMenuRequest,
    },
    platform::status_item::{PlatformStatusItem, PlatformStatusItemManager},
    Context, EngineHandle, MenuDelegate, MethodCallHandler, MethodInvokerProvider, Point, Rect,
    RegisteredMethodCallHandler,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StatusItemHandle(pub(crate) i64);

pub trait StatusItemDelegate {
    fn on_action(&self, handle: StatusItemHandle, action: StatusItemActionType, position: Point);
}

pub struct StatusItemManager {
    context: Context,
    status_item_map: HashMap<StatusItemHandle, Rc<PlatformStatusItem>>,
    platform_manager: Rc<PlatformStatusItemManager>,
    next_handle: StatusItemHandle,
    weak_self: Late<Weak<RefCell<StatusItemManager>>>,
    invoker_provider: Late<MethodInvokerProvider>,
}

impl StatusItemManager {
    pub(super) fn new(context: Context) -> RegisteredMethodCallHandler<Self> {
        let platform_manager = Rc::new(PlatformStatusItemManager::new(context.clone()));
        platform_manager.assign_weak_self(Rc::downgrade(&platform_manager));
        Self {
            context: context.clone(),
            status_item_map: HashMap::new(),
            next_handle: StatusItemHandle(1),
            platform_manager,
            weak_self: Late::new(),
            invoker_provider: Late::new(),
        }
        .register(context, channel::STATUS_ITEM_MANAGER)
    }

    fn init(&mut self, engine: EngineHandle) {
        // Remove all status items from this engine (useful for hot restart)
        let items: Vec<StatusItemHandle> = self
            .status_item_map
            .iter()
            .filter_map(|(handle, i)| {
                if i.engine == engine {
                    Some(*handle)
                } else {
                    None
                }
            })
            .collect();
        for handle in items {
            let item = self.status_item_map.remove(&handle);
            if let Some(item) = item {
                self.platform_manager.unregister_status_item(&item)
            }
        }
    }

    fn on_create(
        &mut self,
        _request: StatusItemCreateRequest,
        engine: EngineHandle,
    ) -> Result<StatusItemHandle> {
        let handle = self.next_handle;
        self.next_handle.0 += 1;

        let status_item = self
            .platform_manager
            .create_status_item(handle, self.weak_self.clone(), engine)
            .map_err(Error::from)?;
        status_item.assign_weak_self(Rc::downgrade(&status_item));

        self.status_item_map.insert(handle, status_item);
        Ok(handle)
    }

    fn get_platform_status_item(&self, item: StatusItemHandle) -> Result<Rc<PlatformStatusItem>> {
        self.status_item_map
            .get(&item)
            .cloned()
            .ok_or(Error::InvalidStatusItemHandle)
    }

    fn set_image(&self, request: StatusItemSetImageRequest) -> Result<()> {
        let item = self.get_platform_status_item(request.handle)?;
        item.set_image(request.image).map_err(Error::from)
    }

    fn set_hint(&self, request: StatusItemSetHintRequest) -> Result<()> {
        let item = self.get_platform_status_item(request.handle)?;
        item.set_hint(request.hint).map_err(Error::from)
    }

    fn set_highlighted(&self, request: StatusItemSetHighlightedRequest) -> Result<()> {
        let item = self.get_platform_status_item(request.handle)?;
        item.set_highlighted(request.highlighted)
            .map_err(Error::from)
    }

    fn show_menu<F>(&self, request: StatusItemShowMenuRequest, on_done: F)
    where
        F: FnOnce(Result<()>) + 'static,
    {
        if let Some(context) = self.context.get() {
            let menu = context
                .menu_manager
                .borrow()
                .borrow()
                .get_platform_menu(request.menu);

            match menu {
                Ok(menu) => {
                    let item = self.get_platform_status_item(request.handle);
                    match item {
                        Ok(item) => item.show_menu(menu, request.offset, move |res| {
                            on_done(res.map_err(Error::from));
                        }),
                        Err(err) => on_done(Err(err)),
                    }
                }
                Err(err) => on_done(Err(err)),
            }
        } else {
            on_done(Err(Error::InvalidContext))
        }
    }

    fn get_geometry(&self, request: StatusItemGetGeometryRequest) -> Result<Rect> {
        let item = self.get_platform_status_item(request.handle)?;
        item.get_geometry().map_err(Error::from)
    }

    fn get_screen_id(&self, request: StatusItemGetScreenIdRequest) -> Result<i64> {
        let item = self.get_platform_status_item(request.handle)?;
        item.get_screen_id().map_err(Error::from)
    }

    fn map_result<T>(result: Result<T>) -> MethodCallResult<Value>
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }
}

impl MethodCallHandler for StatusItemManager {
    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::status_item::INIT => {
                self.init(engine);
                reply.send_ok(Value::Null);
            }
            method::status_item::CREATE => {
                let request: StatusItemCreateRequest = from_value(&call.args).unwrap();
                let res = self.on_create(request, engine);
                reply.send(Self::map_result(res));
            }
            method::status_item::DESTROY => {
                let request: StatusItemDestroyRequest = from_value(&call.args).unwrap();
                let item = self.status_item_map.remove(&request.handle);
                if let Some(item) = item {
                    self.platform_manager.unregister_status_item(&item);
                }
                reply.send_ok(Value::Null);
            }
            method::status_item::SET_IMAGE => {
                let request: StatusItemSetImageRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.set_image(request)));
            }
            method::status_item::SET_HINT => {
                let request: StatusItemSetHintRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.set_hint(request)));
            }
            method::status_item::SHOW_MENU => {
                let request: StatusItemShowMenuRequest = from_value(&call.args).unwrap();
                self.show_menu(request, |res| reply.send(Self::map_result(res)));
            }
            method::status_item::SET_HIGHLIGHTED => {
                let request: StatusItemSetHighlightedRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.set_highlighted(request)));
            }
            method::status_item::GET_GEOMETRY => {
                let request: StatusItemGetGeometryRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.get_geometry(request)));
            }
            method::status_item::GET_SCREEN_ID => {
                let request: StatusItemGetScreenIdRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.get_screen_id(request)));
            }
            _ => {}
        }
    }

    fn assign_weak_self(&mut self, weak_self: Weak<RefCell<Self>>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker_provider(&mut self, provider: MethodInvokerProvider) {
        self.invoker_provider.set(provider);
    }
}

impl StatusItemDelegate for StatusItemManager {
    fn on_action(&self, handle: StatusItemHandle, action: StatusItemActionType, position: Point) {
        let item = self.status_item_map.get(&handle);
        if let Some(item) = item {
            let invoker = self
                .invoker_provider
                .get_method_invoker_for_engine(item.engine);
            invoker
                .call_method(
                    method::status_item::ON_ACTION,
                    to_value(&StatusItemAction {
                        handle,
                        action,
                        position,
                    })
                    .unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }
}
