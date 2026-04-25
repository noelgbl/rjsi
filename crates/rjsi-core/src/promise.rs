use crate::{
    FromJsValue, IntoJsValue, JsContext, JsContextImpl, JsEngine, JsFunc, JsObject, JsObjectOps,
    JsResult, JsTypeOf, JsValue, JsValueImpl, RjsiJSError,
};
use std::ops::Deref;

/// Type alias for the return value of `promise()`.
type PromiseResult<'js, E> = Result<(Promise<'js, E>, JsFunc<'js, E>, JsFunc<'js, E>), RjsiJSError>;

/// Thin wrapper around a JavaScript Promise object.
pub struct Promise<'js, E: JsEngine> {
    obj: JsObject<'js, E>,
}

impl<'js, E: JsEngine> Clone for Promise<'js, E> {
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
        }
    }
}

impl<'js, E: JsEngine> Deref for Promise<'js, E> {
    type Target = JsObject<'js, E>;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for Promise<'js, E>
where
    E::Value: JsValueImpl,
{
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.obj.into_js_value(_ctx)
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for Promise<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        let obj = JsObject::from_js_value(ctx, value)?;
        Ok(Self { obj })
    }
}

impl<'js, E: JsEngine> JsContext<'js, E> {
    /// Creates a new JavaScript Promise and returns the Promise along with its resolve and reject functions.
    pub fn promise(self) -> PromiseResult<'js, E>
    where
        E::Value: JsTypeOf,
    {
        let (p, res, rej) = JsContextImpl::promise(self.native_context());
        let promise = JsObject::from_js_value(self.clone(), JsValue::from_raw(self.clone(), p))?;
        let resolver = <JsFunc<'js, E> as FromJsValue<'js, E>>::from_js_value(
            self.clone(),
            JsValue::from_raw(self.clone(), res),
        )?;
        let reject = <JsFunc<'js, E> as FromJsValue<'js, E>>::from_js_value(
            self.clone(),
            JsValue::from_raw(self.clone(), rej),
        )?;
        Ok((Promise { obj: promise }, resolver, reject))
    }
}

impl<'js, E: JsEngine> Promise<'js, E> {
    pub fn new(ctx: JsContext<'js, E>) -> PromiseResult<'js, E>
    where
        E::Value: JsTypeOf,
    {
        ctx.promise()
    }

    pub fn then(&self) -> JsResult<JsFunc<'js, E>>
    where
        E::Value: JsTypeOf + JsObjectOps,
    {
        self.obj.get("then")
    }

    pub fn catch(&self) -> JsResult<JsFunc<'js, E>>
    where
        E::Value: JsTypeOf + JsObjectOps,
    {
        self.obj.get("catch")
    }

    pub fn into_object(self) -> JsObject<'js, E> {
        self.obj
    }
}
