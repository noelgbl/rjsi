use crate::capabilities::Promise;
use crate::module::{Loader, ModuleHost, Resolver};
use crate::{Engine, Object, PersistentValue, Result, Value};

pub struct Context<'rt, E: Engine> {
    pub(crate) raw: E::Context<'rt>,
}

impl<'rt, E: Engine> Context<'rt, E> {
    pub fn new(raw: E::Context<'rt>) -> Self {
        Self { raw }
    }

    pub fn with_context_mut<R>(&mut self, f: impl FnOnce(&mut E::Context<'rt>) -> R) -> R {
        f(&mut self.raw)
    }

    pub fn eval_with_filename(&mut self, src: &str, filename: &str) -> Result<Value<'rt, E>> {
        E::eval(&mut self.raw, src, Some(filename)).map(Value::new)
    }

    pub fn eval(&mut self, src: &str) -> Result<Value<'rt, E>> {
        E::eval(&mut self.raw, src, None).map(Value::new)
    }

    pub fn globals(&mut self) -> Object<'rt, E> {
        Object::new(E::global_object(&mut self.raw))
    }

    pub fn new_object(&mut self) -> Result<Object<'rt, E>> {
        E::object_new(&mut self.raw).map(Object::new)
    }

    pub fn undefined(&mut self) -> Value<'rt, E> {
        Value::new(E::make_undefined(&mut self.raw))
    }

    pub fn null(&mut self) -> Value<'rt, E> {
        Value::new(E::make_null(&mut self.raw))
    }

    pub fn boolean(&mut self, v: bool) -> Value<'rt, E> {
        Value::new(E::make_bool(&mut self.raw, v))
    }

    pub fn integer(&mut self, v: i32) -> Value<'rt, E> {
        Value::new(E::make_i32(&mut self.raw, v))
    }

    pub fn number(&mut self, v: f64) -> Value<'rt, E> {
        Value::new(E::make_f64(&mut self.raw, v))
    }

    pub fn string(&mut self, s: &str) -> Result<Value<'rt, E>> {
        E::make_string(&mut self.raw, s).map(Value::new)
    }

    pub fn function<F, P>(&mut self, name: &str, func: F) -> Result<crate::Function<'rt, E>>
    where
        F: crate::function::IntoJsFunc<E, P>,
        P: 'static,
    {
        let adapter = crate::function::IntoJsFuncAdapter::<F, P>::new::<E>(func);
        E::make_function(&mut self.raw, name, adapter).map(crate::Function::new)
    }

    pub fn raw_function<F>(&mut self, name: &str, func: F) -> Result<crate::Function<'rt, E>>
    where
        F: crate::args::RawHostFn<E> + 'static,
    {
        E::make_function(&mut self.raw, name, func).map(crate::Function::new)
    }

    pub fn catch_exception(&mut self) -> Option<crate::Value<'rt, E>> {
        E::catch_exception(&mut self.raw).map(crate::Value::new)
    }

    /// Roots `value` until the returned [`PersistentValue`] is dropped.
    pub fn persist_value(&mut self, value: Value<'rt, E>) -> PersistentValue<E> {
        PersistentValue::persist(self, value)
    }
}

pub trait ContextPromiseExt<'rt, E: Engine + crate::capabilities::Promises> {
    fn promise_new(&mut self) -> Result<(crate::Object<'rt, E>, crate::Object<'rt, E>)>;
    fn promise_resolve(
        &mut self,
        resolver: crate::Object<'rt, E>,
        value: crate::Value<'rt, E>,
    ) -> Result<()>;
    fn promise_reject(
        &mut self,
        resolver: crate::Object<'rt, E>,
        reason: crate::Value<'rt, E>,
    ) -> Result<()>;
    fn promise_state(
        &mut self,
        promise: &crate::Object<'rt, E>,
    ) -> Result<crate::capabilities::PromiseState>;
    fn promise_result(
        &mut self,
        promise: &crate::Object<'rt, E>,
    ) -> Result<Option<std::result::Result<crate::Value<'rt, E>, crate::Value<'rt, E>>>>;
}

impl<'rt, E> ContextPromiseExt<'rt, E> for Context<'rt, E>
where
    E: Engine + crate::capabilities::Promises,
{
    fn promise_new(&mut self) -> Result<(crate::Object<'rt, E>, crate::Object<'rt, E>)> {
        let (promise, resolver) = E::promise_new(self)?;
        Ok((crate::Object::new(promise), crate::Object::new(resolver)))
    }

    fn promise_resolve(
        &mut self,
        resolver: crate::Object<'rt, E>,
        value: crate::Value<'rt, E>,
    ) -> Result<()> {
        E::promise_resolve(self, resolver.into_raw(), value.into_raw())
    }

    fn promise_reject(
        &mut self,
        resolver: crate::Object<'rt, E>,
        reason: crate::Value<'rt, E>,
    ) -> Result<()> {
        E::promise_reject(self, resolver.into_raw(), reason.into_raw())
    }

    fn promise_state(
        &mut self,
        promise: &crate::Object<'rt, E>,
    ) -> Result<crate::capabilities::PromiseState> {
        E::promise_state(self, promise.as_raw())
    }

    fn promise_result(
        &mut self,
        promise: &crate::Object<'rt, E>,
    ) -> Result<Option<std::result::Result<crate::Value<'rt, E>, crate::Value<'rt, E>>>> {
        let raw = E::promise_result(self, promise.as_raw())?;
        Ok(raw.map(|r| match r {
            Ok(v) => Ok(crate::Value::new(v)),
            Err(e) => Err(crate::Value::new(e)),
        }))
    }
}

pub trait ContextMicrotaskExt<'rt, E: Engine + crate::capabilities::Microtasks> {
    fn queue_microtask(&mut self, task: E::Function<'rt>);
    fn drain_microtasks(&mut self);
}

impl<'rt, E> ContextMicrotaskExt<'rt, E> for Context<'rt, E>
where
    E: Engine + crate::capabilities::Microtasks,
{
    fn queue_microtask(&mut self, task: E::Function<'rt>) {
        E::queue_microtask(self, task)
    }

    fn drain_microtasks(&mut self) {
        E::drain_microtasks(self)
    }
}

pub trait ContextModulesExt<'rt, E: Engine + crate::capabilities::Modules> {
    fn module_evaluate(&mut self, name: &str, src: &str) -> Result<Promise<'rt, E>>;

    fn module_import(&mut self, specifier: &str) -> Result<Promise<'rt, E>>;
}

impl<'rt, E> ContextModulesExt<'rt, E> for Context<'rt, E>
where
    E: Engine + crate::capabilities::Modules,
{
    fn module_evaluate(&mut self, name: &str, src: &str) -> Result<Promise<'rt, E>> {
        let raw = E::module_evaluate(self, name, src)?;
        Ok(Promise::new(Object::new(raw)))
    }

    fn module_import(&mut self, specifier: &str) -> Result<Promise<'rt, E>> {
        let raw = E::module_import(self, specifier)?;
        Ok(Promise::new(Object::new(raw)))
    }
}

pub trait RuntimeModulesExt<E: Engine + crate::capabilities::Modules> {
    fn install_module_host<R, L>(&mut self, resolver: R, loader: L) -> Result<()>
    where
        R: Resolver,
        L: Loader;

    fn install_module_host_boxed(&mut self, host: ModuleHost) -> Result<()>;

    fn set_import_meta_hook<F>(&mut self, hook: F) -> Result<()>
    where
        F: FnMut(&str) -> std::collections::HashMap<String, String> + 'static;
}

#[doc(hidden)]
pub mod __cx {
    use crate::Engine;

    pub fn context_mut<'rt, 'b, E: Engine>(
        cx: &'b mut super::Context<'rt, E>,
    ) -> &'b mut E::Context<'rt> {
        &mut cx.raw
    }

    pub fn into_context<'rt, E: Engine>(cx: super::Context<'rt, E>) -> E::Context<'rt> {
        cx.raw
    }
}
