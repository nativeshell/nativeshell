use std::{collections::HashMap, rc::Rc};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodCallResult, Value,
    },
    Error, Result,
};

use super::{
    api_constants::{channel, method},
    api_model::{
        StatusItemCreateRequest, StatusItemDestroyRequest, StatusItemSetImageRequest,
        StatusItemSetMenuRequest,
    },
    platform::status_item::PlatformStatusItem,
    Context, EngineHandle, MenuDelegate, MethodCallHandler, RegisteredMethodCallHandler,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StatusItemHandle(pub(crate) i64);

pub struct StatusItemManager {
    context: Context,
    status_item_map: HashMap<StatusItemHandle, Rc<PlatformStatusItem>>,
    next_handle: StatusItemHandle,
}

impl StatusItemManager {
    pub(super) fn new(context: Context) -> RegisteredMethodCallHandler<Self> {
        Self {
            context: context.clone(),
            status_item_map: HashMap::new(),
            next_handle: StatusItemHandle(1),
        }
        .register(context, channel::STATUS_ITEM_MANAGER)
    }

    fn on_create(
        &mut self,
        _request: StatusItemCreateRequest,
        engine: EngineHandle,
    ) -> Result<StatusItemHandle> {
        let handle = self.next_handle;
        self.next_handle.0 += 1;

        let status_item = Rc::new(PlatformStatusItem::new(
            self.context.clone(),
            handle,
            engine,
        ));
        status_item.assign_weak_self(Rc::downgrade(&status_item));

        self.status_item_map.insert(handle, status_item);
        Ok(handle)
    }

    fn get_platform_status_item(&self, item: StatusItemHandle) -> Result<Rc<PlatformStatusItem>> {
        self.status_item_map
            .get(&item)
            .map(|item| item.clone())
            .ok_or(Error::InvalidStatusItemHandle)
    }

    fn set_image(&self, request: StatusItemSetImageRequest) -> Result<()> {
        let item = self.get_platform_status_item(request.handle)?;
        item.set_image(request.image);
        Ok(())
    }

    fn set_menu(&self, request: StatusItemSetMenuRequest) -> Result<()> {
        if let Some(context) = self.context.get() {
            let menu = match request.menu {
                Some(menu) => Some(
                    context
                        .menu_manager
                        .borrow()
                        .borrow()
                        .get_platform_menu(menu)?,
                ),
                None => None,
            };
            let item = self.get_platform_status_item(request.handle)?;
            item.set_menu(menu);
            Ok(())
        } else {
            Err(Error::InvalidContext)
        }
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
            method::status_item::CREATE => {
                let request: StatusItemCreateRequest = from_value(&call.args).unwrap();
                let res = self.on_create(request, engine);
                reply.send(Self::map_result(res));
            }
            method::status_item::DESTROY => {
                let request: StatusItemDestroyRequest = from_value(&call.args).unwrap();
                self.status_item_map.remove(&request.handle);
                reply.send_ok(Value::Null);
            }
            method::status_item::SET_IMAGE => {
                let request: StatusItemSetImageRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.set_image(request)));
            }
            method::status_item::SET_MENU => {
                let request: StatusItemSetMenuRequest = from_value(&call.args).unwrap();
                reply.send(Self::map_result(self.set_menu(request)));
            }
            _ => {}
        }
    }
}
