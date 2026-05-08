use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use boa_engine::object::{FunctionObjectBuilder, JsObject, ObjectInitializer};
use boa_engine::property::PropertyKey as BoaPropertyKey;
use boa_engine::script::Script;
use boa_engine::{
    Context as BoaCx, JsResult as BoaJsResult, JsString, JsSymbol, JsValue, NativeFunction, Source,
};
use rjsi_core::{Engine, Error, PropertyKey, Result};

thread_local! {
    static PENDING_BOA_JS_ERROR: RefCell<Option<boa_engine::JsError>> =
        const { RefCell::new(None) };
}

pub struct BoaEngine;

pub struct BoaContext<'rt> {
    pub(crate) inner: &'rt mut BoaCx,
    pub(crate) runtime: *mut crate::runtime::BoaRuntime,
}

impl<'rt> Deref for BoaContext<'rt> {
    type Target = BoaCx;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'rt> DerefMut for BoaContext<'rt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

pub struct BoaArgs {
    pub(crate) argv: Vec<JsValue>,
}

pub(crate) fn map_js<T>(_cx: &mut BoaCx, res: BoaJsResult<T>) -> Result<T> {
    res.map_err(|e| {
        PENDING_BOA_JS_ERROR.with(|slot| {
            *slot.borrow_mut() = Some(e);
        });
        Error::Exception
    })
}

fn property_key<'cx>(
    cx: &mut BoaContext<'cx>,
    key: PropertyKey<'cx, BoaEngine>,
) -> Result<BoaPropertyKey> {
    match key {
        PropertyKey::Str(s) => Ok(JsString::from(s).into()),
        PropertyKey::Prepared(k) => Ok(crate::runtime::prepared_key(cx, &k)?.into()),
        PropertyKey::Symbol(s) => Ok(s.into()),
        PropertyKey::Index(i) => Ok(i.into()),
    }
}

impl Engine for BoaEngine {
    const ENGINE_NAME: &str = "Boa";

    type Runtime = crate::runtime::BoaRuntime;
    type Context<'rt> = BoaContext<'rt>;
    type Scope<'cx> = ();
    type Value<'cx> = JsValue;
    type Object<'cx> = JsObject;
    type Function<'cx> = JsObject;
    type String<'cx> = JsString;
    type Symbol<'cx> = JsSymbol;
    type Key<'cx> = JsString;
    type PreparedKeyData = JsString;
    type RawArgs<'cx> = BoaArgs;
    type PersistentValue = JsValue;

    fn enter<'rt>(runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        let runtime_ptr = runtime as *mut _;
        BoaContext {
            inner: &mut runtime.context,
            runtime: runtime_ptr,
        }
    }

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        args.argv.get(index).cloned()
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        filename: Option<&str>,
    ) -> Result<Self::Value<'rt>> {
        let source = match filename {
            Some(path) => Source::from_reader(src.as_bytes(), Some(Path::new(path))),
            None => Source::from_bytes(src),
        };
        let parsed = Script::parse(source, None, cx.deref_mut());
        let script = map_js(cx.deref_mut(), parsed)?;
        let evaluated = script.evaluate(cx.deref_mut());
        map_js(cx.deref_mut(), evaluated)
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        cx.global_object().clone()
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> Result<Self::Object<'rt>> {
        let obj = ObjectInitializer::new(cx.deref_mut()).build();
        Ok(obj)
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<Self::Value<'rt>> {
        let k = property_key(cx, key)?;
        let res = obj.get(k, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> Result<()> {
        let k = property_key(cx, key)?;
        let res = obj.set(k, val, true, cx.deref_mut());
        map_js(cx.deref_mut(), res)?;
        Ok(())
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let k = property_key(cx, key)?;
        let res = obj.has_property(k, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let k = property_key(cx, key)?;
        let res = obj.delete_property_or_throw(k, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> Result<Self::Value<'rt>> {
        let res = func.call(&this, args, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_undefined()
    }

    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_null()
    }

    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_boolean()
    }

    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_number()
    }

    fn value_is_string<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_string()
    }

    fn value_is_object<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_object()
    }

    fn value_is_function<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_callable()
    }

    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool {
        val.as_object().is_some_and(|o| o.is_array())
    }

    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_symbol()
    }

    fn value_is_bigint<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_bigint()
    }

    fn make_undefined<'rt>(_: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        JsValue::undefined()
    }

    fn make_null<'rt>(_: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        JsValue::null()
    }

    fn make_bool<'rt>(_: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        JsValue::from(v)
    }

    fn make_i32<'rt>(_: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        JsValue::from(v)
    }

    fn make_f64<'rt>(_: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        JsValue::rational(v)
    }

    fn make_string<'rt>(_: &mut Self::Context<'rt>, s: &str) -> Result<Self::Value<'rt>> {
        Ok(JsValue::from(JsString::from(s)))
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'rt>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        let realm = cx.deref_mut().realm().clone();
        let func_cell = std::cell::RefCell::new(func);

        let native = unsafe {
            NativeFunction::from_closure(move |this, args, boa_cx: &mut BoaCx| {
                let wrapper = BoaContext {
                    inner: boa_cx,
                    runtime: std::ptr::null_mut(),
                };
                let mut rjsi_cx = rjsi_core::Context::new(wrapper);
                let scope = rjsi_core::Scope::new(&mut rjsi_cx);
                let mut callback_cx = rjsi_core::CallbackCx::new(scope);

                let this_core = rjsi_core::Value::new(this.clone());
                let argv: Vec<JsValue> = args.to_vec();
                let args_core = rjsi_core::Args::new(BoaArgs { argv });

                let res = func_cell
                    .borrow_mut()
                    .call(&mut callback_cx, this_core, args_core);

                match res {
                    Ok(v) => Ok(v.into_raw()),
                    Err(Error::Exception) => {
                        let err = PENDING_BOA_JS_ERROR.with(|slot| slot.borrow_mut().take());
                        Err(err.unwrap_or_else(|| {
                            boa_engine::JsNativeError::error()
                                .with_message("JavaScript raised an exception")
                                .into()
                        }))
                    }
                    Err(e) => Err(boa_engine::JsNativeError::error()
                        .with_message(e.to_string())
                        .into()),
                }
            })
        };

        let js_fn = FunctionObjectBuilder::new(&realm, native)
            .name(JsString::from(name))
            .length(0)
            .constructor(false)
            .build();

        Ok(js_fn.into())
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> {
        val.as_boolean()
    }

    fn value_to_f64<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> Result<f64> {
        let n = val.to_number(cx.deref_mut());
        map_js(cx.deref_mut(), n)
    }

    fn value_to_string_utf8<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> Result<String> {
        let s = val.to_string(cx.deref_mut());
        let s = map_js(cx.deref_mut(), s)?;
        Ok(s.to_std_string_lossy())
    }

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx> {
        obj.into()
    }

    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>> {
        val.as_object().map(|o| o.clone())
    }

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx> {
        f.into()
    }

    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        val.as_object()
            .filter(|o| o.is_callable())
            .map(|o| o.clone())
    }

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx> {
        f
    }

    fn persist_value<'rt>(
        _cx: &mut Self::Context<'rt>,
        val: Self::Value<'rt>,
    ) -> Self::PersistentValue {
        val
    }

    fn restore_value<'rt>(
        _cx: &mut Self::Context<'rt>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'rt>> {
        Ok(persisted.clone())
    }
}

impl rjsi_core::capabilities::Promises for BoaEngine {
    type PromiseResolver<'cx> = boa_engine::builtins::promise::ResolvingFunctions;

    fn promise_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::PromiseResolver<'rt>)> {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let (promise, resolvers) = boa_engine::object::builtins::JsPromise::new_pending(boa_cx);
        Ok((
            boa_engine::JsValue::from(promise)
                .as_object()
                .unwrap()
                .clone(),
            resolvers,
        ))
    }

    fn promise_resolve<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::PromiseResolver<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()> {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let res = resolver
            .resolve
            .call(&boa_engine::JsValue::undefined(), &[value], boa_cx);
        map_js(boa_cx, res)?;
        Ok(())
    }

    fn promise_reject<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::PromiseResolver<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()> {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let res = resolver
            .reject
            .call(&boa_engine::JsValue::undefined(), &[reason], boa_cx);
        map_js(boa_cx, res)?;
        Ok(())
    }
}

impl rjsi_core::capabilities::Microtasks for BoaEngine {
    fn queue_microtask<'rt>(cx: &mut rjsi_core::Context<'rt, Self>, task: Self::Function<'rt>) {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let promise = boa_cx
            .global_object()
            .get(boa_engine::JsString::from("Promise"), boa_cx)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        let resolve = promise
            .get(boa_engine::JsString::from("resolve"), boa_cx)
            .unwrap()
            .as_callable()
            .unwrap()
            .clone();
        let resolved = resolve
            .call(&boa_engine::JsValue::undefined(), &[], boa_cx)
            .unwrap();
        let then = resolved
            .as_object()
            .unwrap()
            .get(boa_engine::JsString::from("then"), boa_cx)
            .unwrap()
            .as_callable()
            .unwrap()
            .clone();
        then.call(&resolved, &[task.into()], boa_cx).unwrap();
    }

    fn drain_microtasks<'rt>(cx: &mut rjsi_core::Context<'rt, Self>) {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let _ = boa_cx.run_jobs();
    }
}
