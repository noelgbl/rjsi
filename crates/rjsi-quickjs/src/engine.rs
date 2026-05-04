use rjsi_core::{Engine, JsError, JsResult, PropertyKey};
use rquickjs::{
    Atom, Ctx, Error as QError, Function, Object, String as QString, Symbol as QSymbol, Value
};

pub struct QuickJsEngine;

pub struct QuickJsContext<'js> {
    pub(crate) qctx: Ctx<'js>,
    pub(crate) runtime: *mut crate::runtime::QuickJsRuntime,
}

impl<'js> QuickJsContext<'js> {
    pub(crate) fn clone_ctx(&self) -> Ctx<'js> {
        self.qctx.clone()
    }
}

pub struct QuickJsArgs<'js> {
    pub(crate) argv: Vec<Value<'js>>,
}

pub(crate) fn map_err<'rt, T>(
    cx: &QuickJsContext<'rt>,
    res: rquickjs::Result<T>,
) -> JsResult<'rt, QuickJsEngine, T> {
    match res {
        Ok(v) => Ok(v),
        Err(QError::Exception) => {
            let ex = cx.qctx.catch();
            Err(JsError::Exception(ex))
        }
        Err(e) => Err(JsError::Host(Box::new(e))),
    }
}

impl Engine for QuickJsEngine {
    type Runtime = crate::runtime::QuickJsRuntime;
    type Context<'rt> = QuickJsContext<'rt>;
    type Scope<'cx> = ();
    type Value<'cx> = Value<'cx>;
    type Object<'cx> = Object<'cx>;
    type Function<'cx> = Function<'cx>;
    type String<'cx> = QString<'cx>;
    type Symbol<'cx> = QSymbol<'cx>;
    type Key<'cx> = Atom<'cx>;
    type PreparedKeyData = crate::runtime::QuickJsPreparedKeyData;
    type Error<'cx> = QError;
    type RawArgs<'cx> = QuickJsArgs<'cx>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with_scope instead for QuickJS")
    }

    fn raw_args_len<'rt>(args: &Self::RawArgs<'rt>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'rt>(args: &Self::RawArgs<'rt>, index: usize) -> Option<Self::Value<'rt>> {
        args.argv.get(index).cloned()
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        _filename: Option<&str>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let res: rquickjs::Result<Value<'_>> = cx.qctx.eval(src);
        map_err(cx, res)
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        cx.qctx.globals()
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> JsResult<'rt, Self, Self::Object<'rt>> {
        let res = Object::new(cx.clone_ctx());
        map_err(cx, res)
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let res: rquickjs::Result<Value<'_>> = match key {
            PropertyKey::Str(s) => obj.get(s),
            PropertyKey::Prepared(k) => obj.get(crate::runtime::prepared_key(cx, &k)?),
            PropertyKey::Symbol(s) => obj.get(s),
            PropertyKey::Index(i) => obj.get(i),
        };
        map_err(cx, res)
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> JsResult<'rt, Self, ()> {
        let val_local: Value<'_> = val;
        let res: rquickjs::Result<()> = match key {
            PropertyKey::Str(s) => obj.set(s, val_local),
            PropertyKey::Prepared(k) => obj.set(crate::runtime::prepared_key(cx, &k)?, val_local),
            PropertyKey::Symbol(s) => obj.set(s, val_local),
            PropertyKey::Index(i) => obj.set(i, val_local),
        };
        map_err(cx, res)
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => obj.contains_key(s),
            PropertyKey::Prepared(k) => obj.contains_key(crate::runtime::prepared_key(cx, &k)?),
            PropertyKey::Symbol(s) => obj.contains_key(s),
            PropertyKey::Index(i) => obj.contains_key(i),
        };
        map_err(cx, res)
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => {
                let _ = obj.remove(s);
                Ok(true)
            }
            PropertyKey::Prepared(k) => {
                let _ = obj.remove(crate::runtime::prepared_key(cx, &k)?);
                Ok(true)
            }
            PropertyKey::Symbol(s) => {
                let _ = obj.remove(s);
                Ok(true)
            }
            PropertyKey::Index(i) => {
                let _ = obj.remove(i);
                Ok(true)
            }
        };
        map_err(cx, res)
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let func_local: Function<'_> = func.clone();
        let this_local: Value<'_> = this;
        let mut fargs = rquickjs::function::Args::new(cx.clone_ctx(), args.len());
        let _ = fargs.this(this_local);
        for a in args {
            fargs.push_arg(a.clone()).unwrap();
        }
        let res: rquickjs::Result<Value<'_>> = func_local.call_arg(fargs);
        map_err(cx, res)
    }

    fn value_is_undefined<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_undefined()
    }
    fn value_is_null<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_null()
    }
    fn value_is_boolean<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_bool()
    }
    fn value_is_number<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_number()
    }
    fn value_is_string<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_string()
    }
    fn value_is_object<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_object()
    }
    fn value_is_function<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_function()
    }
    fn value_is_array<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_array()
    }
    fn value_is_symbol<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_symbol()
    }
    fn value_is_bigint<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_big_int()
    }

    fn make_undefined<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::new_undefined(cx.clone_ctx())
    }
    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::new_null(cx.clone_ctx())
    }
    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        Value::new_bool(cx.clone_ctx(), v)
    }
    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        Value::new_int(cx.clone_ctx(), v)
    }
    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        Value::new_float(cx.clone_ctx(), v)
    }

    fn make_string<'rt>(
        cx: &mut Self::Context<'rt>,
        s: &str,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let res = QString::from_str(cx.clone_ctx(), s).map(|s| s.into_value());
        map_err(cx, res)
    }

    fn value_to_bool<'rt>(val: &Self::Value<'rt>) -> Option<bool> {
        val.as_bool()
    }

    fn value_to_f64<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, f64> {
        let res: rquickjs::Result<f64> = val.clone().get();
        map_err(cx, res)
    }

    fn value_to_string_utf8<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, std::string::String> {
        let s: rquickjs::Result<std::string::String> = val.clone().get();
        map_err(cx, s)
    }

    fn object_to_value<'rt>(obj: Self::Object<'rt>) -> Self::Value<'rt> {
        obj.into_value()
    }
    fn value_to_object<'rt>(val: Self::Value<'rt>) -> Option<Self::Object<'rt>> {
        val.into_object()
    }
    fn function_to_value<'rt>(f: Self::Function<'rt>) -> Self::Value<'rt> {
        f.into_value()
    }
    fn value_to_function<'rt>(val: Self::Value<'rt>) -> Option<Self::Function<'rt>> {
        val.into_function()
    }
    fn function_to_object<'rt>(f: Self::Function<'rt>) -> Self::Object<'rt> {
        f.into_value().into_object().unwrap()
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> JsResult<'rt, Self, Self::Function<'rt>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        fn call_host_fn<'js, F: rjsi_core::RawHostFn<QuickJsEngine> + 'static>(
            func_cell: &std::cell::RefCell<F>,
            runtime: *mut crate::runtime::QuickJsRuntime,
            ctx: rquickjs::Ctx<'js>,
            this: rquickjs::function::This<rquickjs::Value<'js>>,
            args: rquickjs::function::Rest<rquickjs::Value<'js>>,
        ) -> rquickjs::Result<rquickjs::Value<'js>> {
            let mut context = rjsi_core::Context::new(QuickJsContext {
                qctx: ctx.clone(),
                runtime,
            });
            let scope = rjsi_core::Scope::new(&mut context);
            let mut callback_cx = rjsi_core::CallbackCx::new(scope);

            let this_core = rjsi_core::Value::new(this.0);
            let args_core = rjsi_core::Args::new(QuickJsArgs { argv: args.0 });

            match func_cell
                .borrow_mut()
                .call(&mut callback_cx, this_core, args_core)
            {
                Ok(v) => Ok(v.into_raw()),
                Err(rjsi_core::JsError::Exception(ex)) => {
                    ctx.throw(ex);
                    Err(rquickjs::Error::Exception)
                }
                Err(e) => {
                    let msg = match e {
                        rjsi_core::JsError::Host(h) => h.to_string(),
                        rjsi_core::JsError::TypeError(t) => format!("TypeError: {}", t),
                        rjsi_core::JsError::RangeError(r) => format!("RangeError: {}", r),
                        _ => "Unknown Error".to_string(),
                    };
                    let err = rquickjs::Exception::from_message(ctx.clone(), &msg).unwrap();
                    ctx.throw(err.into_value());
                    Err(rquickjs::Error::Exception)
                }
            }
        }

        let func_cell = std::cell::RefCell::new(func);
        let runtime = cx.runtime;
        let qjs_func = rquickjs::Function::new(cx.clone_ctx(), move |ctx, this, args| {
            call_host_fn(&func_cell, runtime, ctx, this, args)
        });

        if let Ok(f) = &qjs_func {
            if let Some(obj) = f.as_object() {
                let _ = obj.set("name", name);
            }
        }

        map_err(cx, qjs_func)
    }
}

pub struct QuickJsPromiseResolver<'cx> {
    pub resolve: rquickjs::Function<'cx>,
    pub reject: rquickjs::Function<'cx>,
}

impl rjsi_core::capabilities::Promises for QuickJsEngine {
    type PromiseResolver<'cx> = QuickJsPromiseResolver<'cx>;

    fn promise_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
    ) -> JsResult<'rt, Self, (Self::Object<'rt>, Self::PromiseResolver<'rt>)> {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        let (promise, resolve, reject) = qctx.promise().map_err(|e| JsError::Host(Box::new(e)))?;
        Ok((
            promise.into_value().into_object().unwrap(),
            QuickJsPromiseResolver { resolve, reject },
        ))
    }

    fn promise_resolve<'rt>(
        _cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::PromiseResolver<'rt>,
        value: Self::Value<'rt>,
    ) -> JsResult<'rt, Self, ()> {
        resolver
            .resolve
            .call::<_, ()>((value,))
            .map_err(|e| JsError::Host(Box::new(e)))?;
        Ok(())
    }

    fn promise_reject<'rt>(
        _cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::PromiseResolver<'rt>,
        reason: Self::Value<'rt>,
    ) -> JsResult<'rt, Self, ()> {
        resolver
            .reject
            .call::<_, ()>((reason,))
            .map_err(|e| JsError::Host(Box::new(e)))?;
        Ok(())
    }
}

impl rjsi_core::capabilities::Microtasks for QuickJsEngine {
    fn queue_microtask<'rt>(cx: &mut rjsi_core::Context<'rt, Self>, task: Self::Function<'rt>) {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        let promise: rquickjs::Object = qctx.globals().get("Promise").unwrap();
        let resolve: rquickjs::Function = promise.get("resolve").unwrap();
        let resolved: rquickjs::Object = resolve
            .call::<_, rquickjs::Object>((rquickjs::Value::new_undefined(qctx.clone()),))
            .unwrap();
        let then: rquickjs::Function = resolved.get("then").unwrap();
        then.call::<_, ()>((task,)).unwrap();
    }

    fn drain_microtasks<'rt>(cx: &mut rjsi_core::Context<'rt, Self>) {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        while qctx.execute_pending_job() {}
    }
}
