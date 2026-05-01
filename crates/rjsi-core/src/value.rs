use crate::function::Function;
use crate::{Context, Engine, JsError, JsResult, Object};

#[repr(transparent)]
pub struct Value<'cx, E: Engine> {
    pub(crate) raw: E::Value<'cx>,
}

impl<'cx, E: Engine> Value<'cx, E> {
    pub fn new(raw: E::Value<'cx>) -> Self {
        Self { raw }
    }

    pub fn into_raw(self) -> E::Value<'cx> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Value<'cx> {
        &self.raw
    }

    pub fn is_undefined(&self) -> bool {
        E::value_is_undefined(&self.raw)
    }

    pub fn is_null(&self) -> bool {
        E::value_is_null(&self.raw)
    }

    pub fn is_nullish(&self) -> bool {
        self.is_undefined() || self.is_null()
    }

    pub fn is_boolean(&self) -> bool {
        E::value_is_boolean(&self.raw)
    }

    pub fn is_number(&self) -> bool {
        E::value_is_number(&self.raw)
    }

    pub fn is_string(&self) -> bool {
        E::value_is_string(&self.raw)
    }

    pub fn is_object(&self) -> bool {
        E::value_is_object(&self.raw)
    }

    pub fn is_function(&self) -> bool {
        E::value_is_function(&self.raw)
    }

    pub fn is_array(&self) -> bool {
        E::value_is_array(&self.raw)
    }

    pub fn is_symbol(&self) -> bool {
        E::value_is_symbol(&self.raw)
    }

    pub fn is_bigint(&self) -> bool {
        E::value_is_bigint(&self.raw)
    }

    pub fn to_bool(&self) -> Option<bool> {
        E::value_to_bool(&self.raw)
    }

    pub fn to_f64(&self, cx: &mut Context<'cx, E>) -> JsResult<'cx, E, f64> {
        E::value_to_f64(&mut cx.raw, &self.raw)
    }

    pub fn to_string(&self, cx: &mut Context<'cx, E>) -> JsResult<'cx, E, String> {
        E::value_to_string_utf8(&mut cx.raw, &self.raw)
    }

    pub fn as_object(self) -> Option<Object<'cx, E>> {
        E::value_to_object(self.raw).map(Object::new)
    }

    pub fn as_function(self) -> Option<Function<'cx, E>> {
        E::value_to_function(self.raw).map(Function::new)
    }

    pub fn try_as_object(self) -> JsResult<'cx, E, Object<'cx, E>> {
        self.as_object()
            .ok_or_else(|| JsError::type_err("expected object"))
    }

    pub fn try_as_function(self) -> JsResult<'cx, E, Function<'cx, E>> {
        self.as_function()
            .ok_or_else(|| JsError::type_err("expected function"))
    }
}
