use crate::{
    FromJsValue, HostError, IntoJsValue, JsContext, JsEngine, JsObject, JsResult, JsTypeOf,
    JsValue, JsValueImpl, RjsiJSError,
};
use std::ops::Deref;

#[derive(Hash, PartialEq)]
pub struct JsProxy<'js, E: JsEngine + 'static>(JsObject<'js, E>);

impl<'js, E: JsEngine> Clone for JsProxy<'js, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js, E: JsEngine> Deref for JsProxy<'js, E> {
    type Target = JsObject<'js, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait JsProxyOps: JsTypeOf {
    /// Creates a JavaScript Proxy equivalent to `new Proxy(target, handler)`.
    fn new_proxy(ctx: &Self::Context, target: Self, handler: Self) -> Result<Self, Self>;

    /// Returns the target of a JavaScript Proxy.
    fn proxy_target(&self) -> Result<Self, Self>;
}

impl<'js, E: JsEngine> JsProxy<'js, E>
where
    E::Value: JsProxyOps,
{
    pub fn new(
        ctx: JsContext<'js, E>,
        target: JsObject<'js, E>,
        handler: JsObject<'js, E>,
    ) -> JsResult<Self> {
        let value = E::Value::new_proxy(
            ctx.native_context(),
            target.into_value(),
            handler.into_value(),
        )
        .map_err(|thrown| {
            RjsiJSError::from_thrown_value(ctx.clone(), JsValue::from_raw(ctx.clone(), thrown))
        })?;
        let v = JsValue::from_raw(ctx.clone(), value);
        Self::from_js_value(ctx, v)
    }

    pub fn target(&self) -> JsResult<JsObject<'js, E>> {
        let ctx = self.context();
        let value = self
            .as_value()
            .proxy_target()
            .map_err(|thrown| {
                RjsiJSError::from_thrown_value(ctx.clone(), JsValue::from_raw(ctx.clone(), thrown))
            })?;
        let v = JsValue::from_raw(ctx.clone(), value);
        JsObject::from_js_value(ctx, v)
    }

    pub fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        if obj.is_proxy() {
            Some(Self(obj))
        } else {
            None
        }
    }
}

impl<'js, E: JsEngine> JsProxy<'js, E> {
    pub fn into_value(self) -> E::Value {
        self.0.into_value()
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsProxy<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_proxy() {
            Ok(Self(value.into()))
        } else {
            Err(HostError::not_proxy().into())
        }
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsProxy<'js, E>
where
    E::Value: JsValueImpl,
{
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0.into_js_value(_ctx)
    }
}
