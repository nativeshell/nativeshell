use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodInvoker, Value,
    },
    util::{Late, OkLog},
    Error, Result,
};

use super::{
    api_constants::*,
    api_model::{MenuAction, MenuCreateRequest, MenuDestroyRequest, MenuOpen, SetMenuRequest},
    platform::menu::{PlatformMenu, PlatformMenuManager},
    Context, EngineHandle, MethodCallHandler, MethodInvokerProvider, RegisteredMethodCallHandler,
    WindowMethodCallResult,
};

struct MenuEntry {
    engine: EngineHandle,
    platform_menu: Rc<PlatformMenu>,
}

pub struct MenuManager {
    context: Context,
    platform_menu_map: HashMap<MenuHandle, MenuEntry>,
    platform_menu_manager: Rc<PlatformMenuManager>,
    next_handle: MenuHandle,
    weak_self: Late<Weak<RefCell<MenuManager>>>,
    invoker_provider: Late<MethodInvokerProvider>,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MenuHandle(pub(crate) i64);

pub trait MenuDelegate {
    fn on_menu_open(&self, menu_handle: MenuHandle);
    fn on_menu_action(&self, menu_handle: MenuHandle, id: i64);
    fn get_platform_menu(&self, menu: MenuHandle) -> Result<Rc<PlatformMenu>>;
    fn move_to_previous_menu(&self, menu_handle: MenuHandle);
    fn move_to_next_menu(&self, menu_handle: MenuHandle);
}

impl MenuManager {
    pub(super) fn new(context: Context) -> RegisteredMethodCallHandler<Self> {
        let platform_manager = Rc::new(PlatformMenuManager::new(context.clone()));
        platform_manager.assign_weak_self(Rc::downgrade(&platform_manager));
        Self {
            context: context.clone(),
            platform_menu_map: HashMap::new(),
            platform_menu_manager: platform_manager,
            next_handle: MenuHandle(1),
            weak_self: Late::new(),
            invoker_provider: Late::new(),
        }
        .register(context, channel::MENU_MANAGER)
    }

    pub fn get_platform_menu_manager(&self) -> &PlatformMenuManager {
        &self.platform_menu_manager
    }

    fn on_create_or_update(
        &mut self,
        request: MenuCreateRequest,
        engine: EngineHandle,
    ) -> Result<MenuHandle> {
        let handle = request.handle.unwrap_or_else(|| {
            let res = self.next_handle;
            self.next_handle.0 += 1;
            res
        });
        let entry = self.platform_menu_map.entry(handle);
        let context = self.context.clone();
        let weak_self = self.weak_self.clone();
        let platform_menu = entry
            .or_insert_with(|| {
                let platform_menu = Rc::new(PlatformMenu::new(context, handle, weak_self));
                platform_menu.assign_weak_self(Rc::downgrade(&platform_menu));
                MenuEntry {
                    engine,
                    platform_menu,
                }
            })
            .platform_menu
            .clone();
        platform_menu
            .update_from_menu(request.menu, self)
            .map_err(Error::from)?;

        Ok(handle)
    }

    fn invoker_for_menu(&self, menu_handle: MenuHandle) -> Option<MethodInvoker<Value>> {
        self.platform_menu_map.get(&menu_handle).map(|e| {
            self.invoker_provider
                .get_method_invoker_for_engine(e.engine)
        })
    }

    fn map_result<T>(result: Result<T>) -> WindowMethodCallResult
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }
}

impl MethodCallHandler for MenuManager {
    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::menu::CREATE_OR_UPDATE => {
                let request: MenuCreateRequest = from_value(&call.args).unwrap();
                let res = self.on_create_or_update(request, engine);
                reply.send(Self::map_result(res));
            }
            method::menu::DESTROY => {
                let request: MenuDestroyRequest = from_value(&call.args).unwrap();
                self.platform_menu_map.remove(&request.handle);
                reply.send_ok(Value::Null);
            }
            method::menu::SET_APP_MENU => {
                let request: SetMenuRequest = from_value(&call.args).unwrap();
                match request.handle {
                    Some(handle) => {
                        let menu = self.platform_menu_map.get(&handle);
                        match menu {
                            Some(menu) => reply.send(Self::map_result(
                                self.platform_menu_manager
                                    .set_app_menu(Some(menu.platform_menu.clone()))
                                    .map_err(|e| e.into()),
                            )),
                            None => {
                                reply.send(Self::map_result::<()>(Err(Error::InvalidMenuHandle)));
                            }
                        }
                    }
                    None => reply.send(Self::map_result(
                        self.platform_menu_manager
                            .set_app_menu(None)
                            .map_err(|e| e.into()),
                    )),
                }
            }
            _ => {}
        };
    }

    fn assign_weak_self(&mut self, weak_self: Weak<RefCell<Self>>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker_provider(&mut self, provider: MethodInvokerProvider) {
        self.invoker_provider.set(provider);
    }
}

impl MenuDelegate for MenuManager {
    fn on_menu_action(&self, menu_handle: MenuHandle, id: i64) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu::ON_ACTION,
                    to_value(&MenuAction {
                        handle: menu_handle,
                        id,
                    })
                    .unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }

    fn on_menu_open(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu::ON_OPEN,
                    to_value(&MenuOpen {
                        handle: menu_handle,
                    })
                    .unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }

    fn get_platform_menu(&self, menu: MenuHandle) -> Result<Rc<PlatformMenu>> {
        self.platform_menu_map
            .get(&menu)
            .map(|c| c.platform_menu.clone())
            .ok_or(Error::InvalidMenuHandle)
    }

    fn move_to_previous_menu(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(method::menu_bar::MOVE_TO_PREVIOUS_MENU, Value::Null, |_| {})
                .ok_log();
        }
    }

    fn move_to_next_menu(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(method::menu_bar::MOVE_TO_NEXT_MENU, Value::Null, |_| {})
                .ok_log();
        }
    }
}
