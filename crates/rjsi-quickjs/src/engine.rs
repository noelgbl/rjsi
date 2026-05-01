use rjsi_core::{Engine, JsError, JsResult, PropertyKey};
use rquickjs::{Atom, Ctx, Error as QError, Function, Object, String as QString, Symbol as QSymbol, Value};

pub struct QuickJsEngine;

pub struct QuickJsArgs<'js> {
    pub(crate) argv: Vec<Value<'js>>,
}

pub(crate) fn map_err<'js, T>(cx: &Ctx<'_>, res: rquickjs::Result<T>) -> JsResult<'js, QuickJsEngine, T> {
    match res {
        Ok(v) => Ok(v),
        Err(QError::Exception) => {
            let ex = cx.catch();
            Err(JsError::Exception(unsafe { cast_value(ex) }))
        }
        Err(e) => Err(JsError::Host(Box::new(e))),
    }
}

#[inline(always)]
unsafe fn cast_value<'a, 'b>(v: Value<'a>) -> Value<'b> {
    unsafe { std::mem::transmute(v) }
}

#[inline(always)]
pub(crate) unsafe fn cast_key<'a, 'b>(v: Atom<'a>) -> Atom<'b> {
    unsafe { std::mem::transmute(v) }
}

#[inline(always)]
unsafe fn cast_object<'a, 'b>(v: Object<'a>) -> Object<'b> {
    unsafe { std::mem::transmute(v) }
}

#[inline(always)]
unsafe fn cast_function<'a, 'b>(v: Function<'a>) -> Function<'b> {
    unsafe { std::mem::transmute(v) }
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

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        args.argv.get(index).map(|v| unsafe { cast_value(v.clone()) })
    }

    fn eval<'cx>(
        cx: &mut Self::Context<'_>,
        src: &str,
        _filename: Option<&str>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let res: rquickjs::Result<Value<'_>> = cx.eval(src);
        map_err(cx, res.map(|v| unsafe { cast_value(v) }))
    }

    fn global_object<'cx>(cx: &mut Self::Context<'_>) -> Self::Object<'cx> {
        unsafe { cast_object(cx.globals()) }
    }

    fn object_new<'cx>(cx: &mut Self::Context<'_>) -> JsResult<'cx, Self, Self::Object<'cx>> {
        let res = Object::new(cx.clone());
        map_err(cx, res.map(|o| unsafe { cast_object(o) }))
    }

    fn object_get<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let res: rquickjs::Result<Value<'_>> = match key {
            PropertyKey::Str(s) => obj.get(s),
            PropertyKey::Interned(k) => obj.get(k),
            PropertyKey::Symbol(s) => obj.get(s),
            PropertyKey::Index(i) => obj.get(i),
        };
        map_err(cx, res.map(|v| unsafe { cast_value(v) }))
    }

    fn object_set<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
        val: Self::Value<'cx>,
    ) -> JsResult<'cx, Self, ()> {
        let val_local: Value<'_> = unsafe { cast_value(val) };
        let res: rquickjs::Result<()> = match key {
            PropertyKey::Str(s) => obj.set(s, val_local),
            PropertyKey::Interned(k) => obj.set(k, val_local),
            PropertyKey::Symbol(s) => obj.set(s, val_local),
            PropertyKey::Index(i) => obj.set(i, val_local),
        };
        map_err(cx, res)
    }

    fn object_has<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => obj.contains_key(s),
            PropertyKey::Interned(k) => obj.contains_key(k),
            PropertyKey::Symbol(s) => obj.contains_key(s),
            PropertyKey::Index(i) => obj.contains_key(i),
        };
        map_err(cx, res)
    }

    fn object_delete<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => { let _ = obj.remove(s); Ok(true) },
            PropertyKey::Interned(k) => { let _ = obj.remove(k); Ok(true) },
            PropertyKey::Symbol(s) => { let _ = obj.remove(s); Ok(true) },
            PropertyKey::Index(i) => { let _ = obj.remove(i); Ok(true) },
        };
        map_err(cx, res)
    }

    fn function_call<'cx>(
        cx: &mut Self::Context<'_>,
        func: &Self::Function<'cx>,
        this: Self::Value<'cx>,
        args: &[Self::Value<'cx>],
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let func_local: Function<'_> = unsafe { cast_function(func.clone()) };
        let this_local: Value<'_> = unsafe { cast_value(this) };
        let mut fargs = rquickjs::function::Args::new(cx.clone(), args.len());
        let _ = fargs.this(this_local);
        for a in args {
            let a_local: Value<'_> = unsafe { cast_value(a.clone()) };
            fargs.push_arg(a_local).unwrap();
        }
        let res: rquickjs::Result<Value<'_>> = func_local.call_arg(fargs);
        map_err(cx, res.map(|v| unsafe { cast_value(v) }))
    }

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool { val.is_undefined() }
    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool { val.is_null() }
    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool { val.is_bool() }
    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool { val.is_number() }
    fn value_is_string<'cx>(val: &Self::Value<'cx>) -> bool { val.is_string() }
    fn value_is_object<'cx>(val: &Self::Value<'cx>) -> bool { val.is_object() }
    fn value_is_function<'cx>(val: &Self::Value<'cx>) -> bool { val.is_function() }
    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool { val.is_array() }
    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool { val.is_symbol() }
    fn value_is_bigint<'cx>(val: &Self::Value<'cx>) -> bool { val.is_big_int() }

    fn make_undefined<'cx>(cx: &mut Self::Context<'_>) -> Self::Value<'cx> { unsafe { cast_value(Value::new_undefined(cx.clone())) } }
    fn make_null<'cx>(cx: &mut Self::Context<'_>) -> Self::Value<'cx> { unsafe { cast_value(Value::new_null(cx.clone())) } }
    fn make_bool<'cx>(cx: &mut Self::Context<'_>, v: bool) -> Self::Value<'cx> { unsafe { cast_value(Value::new_bool(cx.clone(), v)) } }
    fn make_i32<'cx>(cx: &mut Self::Context<'_>, v: i32) -> Self::Value<'cx> { unsafe { cast_value(Value::new_int(cx.clone(), v)) } }
    fn make_f64<'cx>(cx: &mut Self::Context<'_>, v: f64) -> Self::Value<'cx> { unsafe { cast_value(Value::new_float(cx.clone(), v)) } }

    fn make_string<'cx>(
        cx: &mut Self::Context<'_>,
        s: &str,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let res = QString::from_str(cx.clone(), s).map(|s| s.into_value());
        map_err(cx, res.map(|v| unsafe { cast_value(v) }))
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> { val.as_bool() }

    fn value_to_f64<'cx>(
        cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, f64> {
        let val_local: Value<'_> = unsafe { cast_value(val.clone()) };
        let res: rquickjs::Result<f64> = val_local.get();
        map_err(cx, res)
    }

    fn value_to_string_utf8<'cx>(
        cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, std::string::String> {
        let val_local: Value<'_> = unsafe { cast_value(val.clone()) };
        let s: rquickjs::Result<std::string::String> = val_local.get();
        map_err(cx, s)
    }

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx> { unsafe { cast_value(obj.into_value()) } }
    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>> { val.into_object().map(|o| unsafe { cast_object(o) }) }
    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx> { unsafe { cast_value(f.into_value()) } }
    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        val.into_function().map(|f| unsafe { cast_function(f) })
    }
    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx> { unsafe { cast_object(f.into_value().into_object().unwrap()) } }

    fn make_function<'cx, F>(
        cx: &mut Self::Context<'_>,
        name: &str,
        func: F,
    ) -> JsResult<'cx, Self, Self::Function<'cx>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        let func_cell = std::cell::RefCell::new(func);
        let qjs_func = rquickjs::Function::new(cx.clone(), move |ctx: rquickjs::Ctx<'_>, this: rquickjs::function::This<rquickjs::Value<'_>>, args: rquickjs::function::Rest<rquickjs::Value<'_>>| -> rquickjs::Result<rquickjs::Value<'_>> {
            let mut context = rjsi_core::Context::new(ctx.clone());
            let scope = rjsi_core::Scope::new(&mut context);
            let mut callback_cx = rjsi_core::CallbackCx::new(scope);

            let this_core =
                rjsi_core::Value::new(unsafe { cast_value(this.0) });

            let mut argv = Vec::with_capacity(args.0.len());
            for a in args.0 {
                argv.push(unsafe { cast_value(a) });
            }

            let args_core = rjsi_core::Args::new(QuickJsArgs { argv });

            let res = func_cell.borrow_mut().call(&mut callback_cx, this_core, args_core);
            match res {
                Ok(v) => Ok(unsafe { cast_value(v.as_raw().clone()) }),
                Err(rjsi_core::JsError::Exception(ex)) => {
                    ctx.throw(unsafe { cast_value(ex) });
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
        });

        if let Ok(f) = &qjs_func {
            if let Some(obj) = f.as_object() {
                let _ = obj.set("name", name);
            }
        }

        map_err(cx, qjs_func.map(|f| unsafe { cast_function(f) }))
    }
}
