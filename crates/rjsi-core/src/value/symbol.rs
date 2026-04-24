use crate::{
    FromJsValue, HostError, IntoJsValue, JsContext, JsEngine, JsObject, JsObjectOps, JsResult,
    JsTypeOf, JsValue, JsValueImpl,
};
use std::ops::Deref;

#[derive(Hash, PartialEq)]
pub struct JsSymbol<'js, E: JsEngine + 'static>(JsObject<'js, E>);

impl<'js, E: JsEngine> Clone for JsSymbol<'js, E>
where
    E::Value: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js, E: JsEngine> JsSymbol<'js, E>
where
    E::Value: JsObjectOps,
{
    pub fn new(ctx: JsContext<'js, E>, descripiton: impl AsRef<str>) -> JsResult<Self> {
        let value = E::Value::create_symbol(ctx.native_context(), descripiton.as_ref());
        if value.is_exception() {
            let v = JsValue::from_raw(ctx.clone(), value);
            Err(crate::RjsiJSError::from_thrown_value(ctx, v))
        } else {
            Ok(Self(JsValue::from_raw(ctx, value).into()))
        }
    }

    pub fn descripiton(&self) -> JsResult<String> {
        self.0.get::<_, String>("description")
    }

    pub fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        if obj.is_symbol() {
            Some(Self(obj))
        } else {
            None
        }
    }
}

impl<'js, E: JsEngine> JsSymbol<'js, E>
where
    E::Value: JsValueImpl,
{
    pub fn into_value(self) -> E::Value {
        self.0.into_value()
    }
}

impl<'js, E: JsEngine> Deref for JsSymbol<'js, E> {
    type Target = JsObject<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsSymbol<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_symbol() {
            Ok(Self(value.into()))
        } else {
            Err(HostError::not_symbol().into())
        }
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsSymbol<'js, E> {
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0.into_js_value(_ctx)
    }
}
