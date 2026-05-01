use crate::{Engine, JsResult, Object, Value};

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

    pub fn eval_with_filename<'cx>(
        &mut self,
        src: &str,
        filename: &str,
    ) -> JsResult<'cx, E, Value<'cx, E>> {
        E::eval(&mut self.raw, src, Some(filename)).map(Value::new)
    }

    pub fn eval<'cx>(&mut self, src: &str) -> JsResult<'cx, E, Value<'cx, E>> {
        E::eval(&mut self.raw, src, None).map(Value::new)
    }

    pub fn globals<'cx>(&mut self) -> Object<'cx, E> {
        Object::new(E::global_object(&mut self.raw))
    }

    pub fn new_object<'cx>(&mut self) -> JsResult<'cx, E, Object<'cx, E>> {
        E::object_new(&mut self.raw).map(Object::new)
    }

    pub fn undefined<'cx>(&mut self) -> Value<'cx, E> {
        Value::new(E::make_undefined(&mut self.raw))
    }

    pub fn null<'cx>(&mut self) -> Value<'cx, E> {
        Value::new(E::make_null(&mut self.raw))
    }

    pub fn boolean<'cx>(&mut self, v: bool) -> Value<'cx, E> {
        Value::new(E::make_bool(&mut self.raw, v))
    }

    pub fn integer<'cx>(&mut self, v: i32) -> Value<'cx, E> {
        Value::new(E::make_i32(&mut self.raw, v))
    }

    pub fn number<'cx>(&mut self, v: f64) -> Value<'cx, E> {
        Value::new(E::make_f64(&mut self.raw, v))
    }

    pub fn string<'cx>(&mut self, s: &str) -> JsResult<'cx, E, Value<'cx, E>> {
        E::make_string(&mut self.raw, s).map(Value::new)
    }

    pub fn function<'cx, F>(
        &mut self,
        name: &str,
        func: F,
    ) -> JsResult<'cx, E, crate::Function<'cx, E>>
    where
        F: crate::args::RawHostFn<E> + 'static,
    {
        E::make_function(&mut self.raw, name, func).map(crate::Function::new)
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
