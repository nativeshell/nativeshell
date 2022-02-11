use std::rc::Rc;

use nativeshell::shell::{
    exec_bundle, register_observatory_listener, Context, ContextOptions, MethodCallHandler,
};
use nativeshell_core::{
    AsyncMethodHandler, GetMessageChannel, IntoPlatformResult, IsolateId,
    MessageChannelDelegate, MethodCallError, MethodHandler, MethodInvoker, PlatformError, Value,
};

nativeshell::include_flutter_plugins!();

struct D {}

impl MessageChannelDelegate for D {
    fn on_isolate_joined(&self, isolate: IsolateId) {
        println!("on_isolate_joined: {}", isolate);
    }

    fn on_message(
        &self,
        isolate: IsolateId,
        message: Value,
        reply: Box<dyn FnOnce(Value) -> bool>,
    ) {
        println!("Message from isolate {}", isolate);
        reply(message);
    }

    fn on_isolate_exited(&self, isolate: IsolateId) {
        println!("on_isolate_exited: {}", isolate);
    }
}

struct MyError {}

impl From<MyError> for PlatformError {
    fn from(_: MyError) -> Self {
        PlatformError {
            code: "n/a".into(),
            message: Some("WooHoo".into()),
            detail: Value::Null,
        }
    }
}

struct T1 {}

impl T1 {
    fn yy(&self) -> Result<Value, PlatformError> {
        let r = self.x();
        // r.map(|e| e.into()).map_err(|e| e.into())
        // Ok(r.into())
        r.into_platform_result()
    }

    fn x(&self) -> Result<String, MyError> {
        Ok("Hi".into())
    }
}

impl MethodHandler for T1 {
    fn assign_invoker(&mut self, invoker: MethodInvoker) {
        invoker.call_method_cv(0, "abc", Value::Null, |e: Result<i64, _>| {

        });
    }

    fn on_method_call(
        &mut self,
        call: nativeshell_core::MethodCall,
        reply: nativeshell_core::MethodCallReply,
    ) {
        // println!("NS: {:?}", call);
        reply.send_ok(call.args);
        // let mut v = Vec::<u8>::new();
        // v.resize(1024 * 1024, 4);
        // reply.send_ok(Value::Null);
        // reply.send(self.x())
    }
}

impl AsyncMethodHandler for T1 {
    fn assign_invoker(&mut self, invoker: nativeshell_core::AsyncMethodInvoker) {
        nativeshell_core::Context::get()
            .run_loop()
            .spawn(async move {
                let res: Result<i64, MethodCallError> =
                    invoker.call_method_cv(0, "hi", Value::Null).await;
            });
    }

    fn on_method_call<'life0, 'async_trait>(
        &'life0 self,
        call: nativeshell_core::MethodCall,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = nativeshell_core::PlatformResult> + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}

impl MethodCallHandler for T1 {
    fn on_method_call(
        &mut self,
        call: nativeshell::codec::MethodCall<nativeshell::codec::Value>,
        reply: nativeshell::codec::MethodCallReply<nativeshell::codec::Value>,
        _engine: nativeshell::shell::EngineHandle,
    ) {
        // println!("FT: {:?}", call);
        reply.send_ok(call.args);
        // reply.send_ok(nativeshell::codec::Value::Null);
        // let mut v = Vec::<u8>::new();
        // v.resize(1024 * 1024, 4);
        // reply.send_ok(nativeshell::codec::Value::U8List(v));
    }
}

fn main() {
    exec_bundle();

    register_observatory_listener("app_template".into());

    env_logger::builder().format_timestamp(None).init();

    let context = Context::new(ContextOptions {
        app_namespace: "AppTemplate".into(),
        flutter_plugins: flutter_get_plugins(),
        ..Default::default()
    });

    let context2 = nativeshell_core::Context::new();
    context2
        .message_channel()
        .register_delegate("abcd", Rc::new(D {}));

    let context = context.unwrap();

    context
        .window_manager
        .borrow_mut()
        .create_window(nativeshell::codec::Value::Null, None)
        .unwrap();

    let _t1 = MethodCallHandler::register(T1 {}, context.weak(), "channel1");
    let _t2 = MethodHandler::register(T1 {}, "channel1");

    context.run_loop.borrow().run();
    context.run_loop.borrow().run();
}
