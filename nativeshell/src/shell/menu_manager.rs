use std::{collections::HashMap, rc::Rc};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodInvoker, Value,
    },
    util::OkLog,
    Error, Result,
};

use super::{
    api_constants::*,
    api_model::{MenuAction, MenuCreateRequest, MenuDestroyRequest, SetMenuRequest},
    platform::menu::{PlatformMenu, PlatformMenuManager},
    Context, EngineHandle, WindowMethodCallResult,
};

struct MenuEntry {
    engine: EngineHandle,
    platform_menu: Rc<PlatformMenu>,
}

pub struct MenuManager {
    context: Rc<Context>,
    platform_menu_map: HashMap<MenuHandle, MenuEntry>,
    platform_menu_manager: PlatformMenuManager,
    next_handle: MenuHandle,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MenuHandle(pub(crate) i64);

impl MenuManager {
    pub(super) fn new(context: Rc<Context>) -> Self {
        let context_copy = context.clone();
        context
            .message_manager
            .borrow_mut()
            .register_method_handler(channel::MENU_MANAGER, move |value, reply, engine| {
                context_copy
                    .menu_manager
                    .borrow_mut()
                    .on_method_call(value, reply, engine);
            });

        Self {
            context: context.clone(),
            platform_menu_map: HashMap::new(),
            platform_menu_manager: PlatformMenuManager::new(context.clone()),
            next_handle: MenuHandle(1),
        }
    }

    pub fn get_platform_menu(&self, menu: MenuHandle) -> Result<Rc<PlatformMenu>> {
        self.platform_menu_map
            .get(&menu)
            .map(|c| c.platform_menu.clone())
            .ok_or(Error::InvalidMenuHandle)
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
            let res = self.next_handle.clone();
            self.next_handle.0 += 1;
            res
        });
        let entry = self.platform_menu_map.entry(handle.clone());
        let context = self.context.clone();
        let platform_menu = entry
            .or_insert_with(|| {
                let platform_menu = Rc::new(PlatformMenu::new(context, handle));
                platform_menu.assign_weak_self(Rc::downgrade(&platform_menu));
                MenuEntry {
                    engine: engine,
                    platform_menu: platform_menu,
                }
            })
            .platform_menu
            .clone();
        platform_menu
            .update_from_menu(request.menu, self)
            .map_err(|e| Error::from(e))?;

        Ok(handle)
    }

    fn invoker_for_menu(&self, menu_handle: MenuHandle) -> Option<MethodInvoker<Value>> {
        self.platform_menu_map.get(&menu_handle).and_then(|e| {
            self.context
                .message_manager
                .borrow()
                .get_method_invoker(e.engine, channel::MENU_MANAGER)
        })
    }

    pub(crate) fn on_menu_action(&self, menu_handle: MenuHandle, id: i64) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu::ON_ACTION.into(),
                    to_value(&MenuAction {
                        handle: menu_handle,
                        id: id,
                    })
                    .unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }

    #[allow(dead_code)] // only used on windows
    pub(crate) fn move_to_previous_menu(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu_bar::MOVE_TO_PREVIOUS_MENU.into(),
                    Value::Null,
                    |_| {},
                )
                .ok_log();
        }
    }

    #[allow(dead_code)] // only used on windows
    pub(crate) fn move_to_next_menu(&self, menu_handle: MenuHandle) {
        if let Some(invoker) = self.invoker_for_menu(menu_handle) {
            invoker
                .call_method(
                    method::menu_bar::MOVE_TO_NEXT_MENU.into(),
                    Value::Null,
                    |_| {},
                )
                .ok_log();
        }
    }

    fn map_result<T>(result: Result<T>) -> WindowMethodCallResult
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }

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
}
