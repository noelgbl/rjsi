use rjsi_core::{Engine, JsError, JsResult, PropertyKey};
use rquickjs::{
    Atom, Ctx, Error as QError, Function, Object, String as QString, Symbol as QSymbol, Value
};

pub struct QuickJsEngine;

pub struct QuickJsArgs<'js> {
    pub(crate) argv: Vec<Value<'js>>,
}

pub(crate) fn map_err<'rt, T>(
    cx: &Ctx<'rt>,
    res: rquickjs::Result<T>,
) -> JsResult<'rt, QuickJsEngine, T> {
    match res {
        Ok(v) => Ok(v),
        Err(QError::Exception) => {
            let ex = cx.catch();
            Err(JsError::Exception(ex))
        }
        Err(e) => Err(JsError::Host(Box::new(e))),
    }
}

impl Engine for QuickJsEngine {
    type Runtime = ();
    type Context<'rt> = Ctx<'rt>;
    type Scope<'cx> = ();
    type Value<'cx> = Value<'cx>;
    type Object<'cx> = Object<'cx>;
    type Function<'cx> = Function<'cx>;
    type String<'cx> = QString<'cx>;
    type Symbol<'cx> = QSymbol<'cx>;
    type Key<'cx> = Atom<'cx>;
    type Error<'cx> = QError;
    type RawArgs<'cx> = QuickJsArgs<'cx>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with instead for QuickJS")
    }

    fn raw_args_len<'rt>(args: &Self::RawArgs<'rt>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'rt>(args: &Self::RawArgs<'rt>, index: usize) -> Option<Self::Value<'rt>> {
        args.argv.get(index).map(|v| v.clone())
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        _filename: Option<&str>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let res: rquickjs::Result<Value<'_>> = cx.eval(src);
        map_err(cx, res.map(|v| v))
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        cx.globals()
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> JsResult<'rt, Self, Self::Object<'rt>> {
        let res = Object::new(cx.clone());
        map_err(cx, res.map(|o| o))
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let res: rquickjs::Result<Value<'_>> = match key {
            PropertyKey::Str(s) => obj.get(s),
            PropertyKey::Interned(k) => obj.get(k),
            PropertyKey::Symbol(s) => obj.get(s),
            PropertyKey::Index(i) => obj.get(i),
        };
        map_err(cx, res.map(|v| v))
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
            PropertyKey::Interned(k) => obj.set(k, val_local),
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
            PropertyKey::Interned(k) => obj.contains_key(k),
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
            PropertyKey::Interned(k) => {
                let _ = obj.remove(k);
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
        let mut fargs = rquickjs::function::Args::new(cx.clone(), args.len());
        let _ = fargs.this(this_local);
        for a in args {
            let a_local: Value<'_> = a.clone();
            fargs.push_arg(a_local).unwrap();
        }
        let res: rquickjs::Result<Value<'_>> = func_local.call_arg(fargs);
        map_err(cx, res.map(|v| v))
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
        Value::new_undefined(cx.clone())
    }
    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::new_null(cx.clone())
    }
    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        Value::new_bool(cx.clone(), v)
    }
    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        Value::new_int(cx.clone(), v)
    }
    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        Value::new_float(cx.clone(), v)
    }

    fn make_string<'rt>(
        cx: &mut Self::Context<'rt>,
        s: &str,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let res = QString::from_str(cx.clone(), s).map(|s| s.into_value());
        map_err(cx, res.map(|v| v))
    }

    fn value_to_bool<'rt>(val: &Self::Value<'rt>) -> Option<bool> {
        val.as_bool()
    }

    fn value_to_f64<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, f64> {
        let val_local: Value<'_> = val.clone();
        let res: rquickjs::Result<f64> = val_local.get();
        map_err(cx, res)
    }

    fn value_to_string_utf8<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, std::string::String> {
        let val_local: Value<'_> = val.clone();
        let s: rquickjs::Result<std::string::String> = val_local.get();
        map_err(cx, s)
    }

    fn object_to_value<'rt>(obj: Self::Object<'rt>) -> Self::Value<'rt> {
        obj.into_value()
    }
    fn value_to_object<'rt>(val: Self::Value<'rt>) -> Option<Self::Object<'rt>> {
        val.into_object().map(|o| o)
    }
    fn function_to_value<'rt>(f: Self::Function<'rt>) -> Self::Value<'rt> {
        f.into_value()
    }
    fn value_to_function<'rt>(val: Self::Value<'rt>) -> Option<Self::Function<'rt>> {
        val.into_function().map(|f| f)
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
            ctx: rquickjs::Ctx<'js>,
            this: rquickjs::function::This<rquickjs::Value<'js>>,
            args: rquickjs::function::Rest<rquickjs::Value<'js>>,
        ) -> rquickjs::Result<rquickjs::Value<'js>> {
            let mut context = rjsi_core::Context::new(ctx.clone());
            let scope = rjsi_core::Scope::new(&mut context);
            let mut callback_cx = rjsi_core::CallbackCx::new(scope);

            let this_core = rjsi_core::Value::new(this.0);

            let mut argv = Vec::with_capacity(args.0.len());
            for a in args.0 {
                argv.push(a);
            }
            let args_core = rjsi_core::Args::new(QuickJsArgs { argv });

            let res = func_cell
                .borrow_mut()
                .call(&mut callback_cx, this_core, args_core);
            match res {
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
        let qjs_func = rquickjs::Function::new(cx.clone(), move |ctx, this, args| {
            call_host_fn(&func_cell, ctx, this, args)
        });

        if let Ok(f) = &qjs_func {
            if let Some(obj) = f.as_object() {
                let _ = obj.set("name", name);
            }
        }

        map_err(cx, qjs_func.map(|f| f))
    }
}
