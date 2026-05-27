use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use boa_engine::object::{FunctionObjectBuilder, JsObject, ObjectInitializer};
use boa_engine::property::PropertyKey as BoaPropertyKey;
use boa_engine::script::Script;
use boa_engine::{
    Context as BoaCx, JsResult as BoaJsResult, JsString, JsSymbol, JsValue, NativeFunction, Source
};
use rjsi_core::{Engine, Error, PropertyKey, Result};

thread_local! {
    static PENDING_BOA_JS_ERROR: RefCell<Option<boa_engine::JsError>> =
        const { RefCell::new(None) };
}

pub struct BoaEngine;

pub struct BoaContext<'js> {
    pub(crate) inner: &'js mut BoaCx,
    pub(crate) runtime: *mut crate::runtime::BoaRuntime,
}

impl<'js> Deref for BoaContext<'js> {
    type Target = BoaCx;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'js> DerefMut for BoaContext<'js> {
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

fn property_key<'js>(
    cx: &mut BoaContext<'js>,
    key: PropertyKey<'js, BoaEngine>,
) -> Result<BoaPropertyKey> {
    match key {
        PropertyKey::Str(s) => Ok(JsString::from(s).into()),
        PropertyKey::Prepared(k) => Ok(crate::runtime::prepared_key(cx, &k)?.into()),
        PropertyKey::Symbol(s) => Ok(s.into_raw().into()),
        PropertyKey::Index(i) => Ok(i.into()),
    }
}

impl Engine for BoaEngine {
    const ENGINE_NAME: &str = "Boa";

    type Runtime = crate::runtime::BoaRuntime;
    type Context<'js> = BoaContext<'js>;
    type Value<'js> = JsValue;
    type Object<'js> = JsObject;
    type Function<'js> = JsObject;
    type String<'js> = JsString;
    type Symbol<'js> = JsSymbol;
    type Key<'js> = JsString;
    type PreparedKeyData = JsString;
    type RawArgs<'js> = BoaArgs;
    type PersistentValue = JsValue;

    fn enter<'js>(runtime: &'js mut Self::Runtime) -> Self::Context<'js> {
        let runtime_ptr = runtime as *mut _;
        BoaContext {
            inner: &mut runtime.context,
            runtime: runtime_ptr,
        }
    }

    fn raw_args_len<'js>(args: &Self::RawArgs<'js>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'js>(args: &Self::RawArgs<'js>, index: usize) -> Option<Self::Value<'js>> {
        args.argv.get(index).cloned()
    }

    fn eval<'js>(
        cx: &mut Self::Context<'js>,
        src: &str,
        filename: Option<&str>,
    ) -> Result<Self::Value<'js>> {
        let source = match filename {
            Some(path) => Source::from_reader(src.as_bytes(), Some(Path::new(path))),
            None => Source::from_bytes(src),
        };
        let parsed = Script::parse(source, None, cx.deref_mut());
        let script = map_js(cx.deref_mut(), parsed)?;
        let evaluated = script.evaluate(cx.deref_mut());
        map_js(cx.deref_mut(), evaluated)
    }

    fn global_object<'js>(cx: &mut Self::Context<'js>) -> Self::Object<'js> {
        cx.global_object().clone()
    }

    fn object_new<'js>(cx: &mut Self::Context<'js>) -> Result<Self::Object<'js>> {
        let obj = ObjectInitializer::new(cx.deref_mut()).build();
        Ok(obj)
    }

    fn object_get<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<Self::Value<'js>> {
        let k = property_key(cx, key)?;
        let res = obj.get(k, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn object_set<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
        val: Self::Value<'js>,
    ) -> Result<()> {
        let k = property_key(cx, key)?;
        let res = obj.set(k, val, true, cx.deref_mut());
        map_js(cx.deref_mut(), res)?;
        Ok(())
    }

    fn object_has<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        let k = property_key(cx, key)?;
        let res = obj.has_property(k, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn object_delete<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        let k = property_key(cx, key)?;
        let res = obj.delete_property_or_throw(k, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn function_call<'js>(
        cx: &mut Self::Context<'js>,
        func: &Self::Function<'js>,
        this: Self::Value<'js>,
        args: &[Self::Value<'js>],
    ) -> Result<Self::Value<'js>> {
        let res = func.call(&this, args, cx.deref_mut());
        map_js(cx.deref_mut(), res)
    }

    fn value_is_undefined<'js>(val: &Self::Value<'js>) -> bool {
        val.is_undefined()
    }

    fn value_is_null<'js>(val: &Self::Value<'js>) -> bool {
        val.is_null()
    }

    fn value_is_boolean<'js>(val: &Self::Value<'js>) -> bool {
        val.is_boolean()
    }

    fn value_is_number<'js>(val: &Self::Value<'js>) -> bool {
        val.is_number()
    }

    fn value_is_string<'js>(val: &Self::Value<'js>) -> bool {
        val.is_string()
    }

    fn value_is_object<'js>(val: &Self::Value<'js>) -> bool {
        val.is_object()
    }

    fn value_is_function<'js>(val: &Self::Value<'js>) -> bool {
        val.is_callable()
    }

    fn value_is_array<'js>(val: &Self::Value<'js>) -> bool {
        val.as_object().is_some_and(|o| o.is_array())
    }

    fn value_is_symbol<'js>(val: &Self::Value<'js>) -> bool {
        val.is_symbol()
    }

    fn value_is_bigint<'js>(val: &Self::Value<'js>) -> bool {
        val.is_bigint()
    }

    fn make_undefined<'js>(_: &mut Self::Context<'js>) -> Self::Value<'js> {
        JsValue::undefined()
    }

    fn make_null<'js>(_: &mut Self::Context<'js>) -> Self::Value<'js> {
        JsValue::null()
    }

    fn make_bool<'js>(_: &mut Self::Context<'js>, v: bool) -> Self::Value<'js> {
        JsValue::from(v)
    }

    fn make_i32<'js>(_: &mut Self::Context<'js>, v: i32) -> Self::Value<'js> {
        JsValue::from(v)
    }

    fn make_f64<'js>(_: &mut Self::Context<'js>, v: f64) -> Self::Value<'js> {
        JsValue::rational(v)
    }

    fn make_string<'js>(_: &mut Self::Context<'js>, s: &str) -> Result<Self::Value<'js>> {
        Ok(JsValue::from(JsString::from(s)))
    }

    fn make_function<'js, F>(
        cx: &mut Self::Context<'js>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'js>>
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
                let this_core = rjsi_core::Value::new(this.clone());
                let argv: Vec<JsValue> = args.to_vec();
                let args_core = rjsi_core::Args::new(BoaArgs { argv });

                let res = func_cell
                    .borrow_mut()
                    .call(&mut rjsi_cx, this_core, args_core);

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

    fn value_as_bool<'js>(val: &Self::Value<'js>) -> Option<bool> {
        val.as_boolean()
    }

    fn value_to_bool<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> bool {
        let _ = cx;
        val.to_boolean()
    }

    fn value_to_f64<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<f64> {
        let n = val.to_number(cx.deref_mut());
        map_js(cx.deref_mut(), n)
    }

    fn value_to_string<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<String> {
        let s = val.to_string(cx.deref_mut());
        let s = map_js(cx.deref_mut(), s)?;
        Ok(s.to_std_string_lossy())
    }

    fn object_to_value<'js>(obj: Self::Object<'js>) -> Self::Value<'js> {
        obj.into()
    }

    fn value_as_object<'js>(val: Self::Value<'js>) -> Option<Self::Object<'js>> {
        val.as_object().map(|o| o.clone())
    }

    fn function_to_value<'js>(f: Self::Function<'js>) -> Self::Value<'js> {
        f.into()
    }

    fn value_as_function<'js>(val: Self::Value<'js>) -> Option<Self::Function<'js>> {
        val.as_object()
            .filter(|o| o.is_callable())
            .map(|o| o.clone())
    }

    fn function_to_object<'js>(f: Self::Function<'js>) -> Self::Object<'js> {
        f
    }

    fn persist_value<'js>(
        _cx: &mut Self::Context<'js>,
        val: Self::Value<'js>,
    ) -> Self::PersistentValue {
        val
    }

    fn restore_value<'js>(
        _cx: &mut Self::Context<'js>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'js>> {
        Ok(persisted.clone())
    }

    fn catch_exception<'js>(cx: &mut Self::Context<'js>) -> Option<Self::Value<'js>> {
        let err = PENDING_BOA_JS_ERROR.with(|slot| slot.borrow_mut().take())?;
        err.into_opaque(cx.inner).ok()
    }

    fn throw<'js>(_cx: &mut Self::Context<'js>, value: Self::Value<'js>) -> Error {
        PENDING_BOA_JS_ERROR.with(|slot| {
            *slot.borrow_mut() = Some(boa_engine::JsError::from_opaque(value));
        });
        Error::Exception
    }
}

impl rjsi_core::capabilities::Promises for BoaEngine {
    fn promise_new<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
    ) -> Result<(Self::Object<'js>, Self::Object<'js>)> {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let (promise, resolvers) = boa_engine::object::builtins::JsPromise::new_pending(boa_cx);
        let promise_obj = boa_engine::JsValue::from(promise)
            .as_object()
            .unwrap()
            .clone();
        let resolver_obj = boa_engine::JsObject::with_null_proto();
        resolver_obj
            .set(
                boa_engine::JsString::from("resolve"),
                boa_engine::JsValue::from(resolvers.resolve),
                false,
                boa_cx,
            )
            .map_err(|_| Error::type_err("failed to install resolve"))?;
        resolver_obj
            .set(
                boa_engine::JsString::from("reject"),
                boa_engine::JsValue::from(resolvers.reject),
                false,
                boa_cx,
            )
            .map_err(|_| Error::type_err("failed to install reject"))?;
        Ok((promise_obj, resolver_obj))
    }

    fn promise_resolve<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        resolver: Self::Object<'js>,
        value: Self::Value<'js>,
    ) -> Result<()> {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let resolve_val = resolver
            .get(boa_engine::JsString::from("resolve"), boa_cx)
            .map_err(|_| Error::type_err("resolver missing `resolve`"))?;
        let resolve = resolve_val
            .as_object()
            .ok_or_else(|| Error::type_err("`resolve` is not a function"))?
            .clone();
        let res = resolve.call(&boa_engine::JsValue::undefined(), &[value], boa_cx);
        map_js(boa_cx, res)?;
        Ok(())
    }

    fn promise_reject<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        resolver: Self::Object<'js>,
        reason: Self::Value<'js>,
    ) -> Result<()> {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let reject_val = resolver
            .get(boa_engine::JsString::from("reject"), boa_cx)
            .map_err(|_| Error::type_err("resolver missing `reject`"))?;
        let reject = reject_val
            .as_object()
            .ok_or_else(|| Error::type_err("`reject` is not a function"))?
            .clone();
        let res = reject.call(&boa_engine::JsValue::undefined(), &[reason], boa_cx);
        map_js(boa_cx, res)?;
        Ok(())
    }

    fn promise_state<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        promise: &Self::Object<'js>,
    ) -> Result<rjsi_core::capabilities::PromiseState> {
        let js_promise = boa_engine::object::builtins::JsPromise::from_object(promise.clone())
            .map_err(|_| Error::type_err("promise_state: object is not a Promise"))?;
        Ok(match js_promise.state() {
            boa_engine::builtins::promise::PromiseState::Pending => {
                rjsi_core::capabilities::PromiseState::Pending
            }
            boa_engine::builtins::promise::PromiseState::Fulfilled(_) => {
                rjsi_core::capabilities::PromiseState::Resolved
            }
            boa_engine::builtins::promise::PromiseState::Rejected(_) => {
                rjsi_core::capabilities::PromiseState::Rejected
            }
        })
    }

    fn promise_result<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        promise: &Self::Object<'js>,
    ) -> Result<Option<std::result::Result<Self::Value<'js>, Self::Value<'js>>>> {
        let js_promise = boa_engine::object::builtins::JsPromise::from_object(promise.clone())
            .map_err(|_| Error::type_err("promise_result: object is not a Promise"))?;
        Ok(match js_promise.state() {
            boa_engine::builtins::promise::PromiseState::Pending => None,
            boa_engine::builtins::promise::PromiseState::Fulfilled(value) => Some(Ok(value)),
            boa_engine::builtins::promise::PromiseState::Rejected(reason) => Some(Err(reason)),
        })
    }
}

impl rjsi_core::capabilities::Microtasks for BoaEngine {
    fn queue_microtask<'js>(cx: &mut rjsi_core::Context<'js, Self>, task: Self::Function<'js>) {
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

    fn drain_microtasks<'js>(cx: &mut rjsi_core::Context<'js, Self>) {
        let boa_cx = rjsi_core::__cx::context_mut(cx);
        let _ = boa_cx.run_jobs();
    }
}
