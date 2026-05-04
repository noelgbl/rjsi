use crate::{Engine, Object, Result, Value};

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

    pub fn function<F>(&mut self, name: &str, func: F) -> Result<crate::Function<'rt, E>>
    where
        F: crate::args::RawHostFn<E> + 'static,
    {
        E::make_function(&mut self.raw, name, func).map(crate::Function::new)
    }
}

pub trait ContextPromiseExt<'rt, E: Engine + crate::capabilities::Promises> {
    fn promise_new(&mut self) -> Result<(crate::Object<'rt, E>, E::PromiseResolver<'rt>)>;
    fn promise_resolve(
        &mut self,
        resolver: E::PromiseResolver<'rt>,
        value: crate::Value<'rt, E>,
    ) -> Result<()>;
    fn promise_reject(
        &mut self,
        resolver: E::PromiseResolver<'rt>,
        reason: crate::Value<'rt, E>,
    ) -> Result<()>;
}

impl<'rt, E> ContextPromiseExt<'rt, E> for Context<'rt, E>
where
    E: Engine + crate::capabilities::Promises,
{
    fn promise_new(&mut self) -> Result<(crate::Object<'rt, E>, E::PromiseResolver<'rt>)> {
        let (obj, resolver) = E::promise_new(self)?;
        Ok((crate::Object::new(obj), resolver))
    }

    fn promise_resolve(
        &mut self,
        resolver: E::PromiseResolver<'rt>,
        value: crate::Value<'rt, E>,
    ) -> Result<()> {
        E::promise_resolve(self, resolver, value.into_raw())
    }

    fn promise_reject(
        &mut self,
        resolver: E::PromiseResolver<'rt>,
        reason: crate::Value<'rt, E>,
    ) -> Result<()> {
        E::promise_reject(self, resolver, reason.into_raw())
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
