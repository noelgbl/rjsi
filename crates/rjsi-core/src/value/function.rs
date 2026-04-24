use crate::function::{
    FromParams, IntoJsCallable, IntoOnceJsCallable, JsParameterType, RustFunc,
};
use crate::{
    Class, FromJsValue, HostError, IntoJsValue, JsContext, JsContextImpl, JsEngine, JsObject,
    JsObjectOps, JsResult, JsTypeOf, JsValue, JsValueImpl, JsValueMapper, PropertyDescriptor,
};
use std::ops::Deref;

mod args;
pub use args::IntoJsArgs;

#[derive(PartialEq, Hash)]
pub struct JsFunc<'js, E: JsEngine + 'static>(JsObject<'js, E>);

impl<'js, E: JsEngine> Deref for JsFunc<'js, E> {
    type Target = JsObject<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> Clone for JsFunc<'js, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsFunc<'js, E> {
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0.into_js_value(_ctx)
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsFunc<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_function() {
            JsObject::from_js_value(ctx, value).map(Self)
        } else {
            Err(HostError::not_function().into())
        }
    }
}

impl<'js, E: JsEngine> JsFunc<'js, E>
where
    E::Value: JsObjectOps,
{
    fn call_with_argv(&self, this: Option<JsObject<'js, E>>, argv: &[E::Value]) -> E::Value {
        let ctx = self.context();
        let this = match this {
            Some(obj) => obj.into_value(),
            None => E::Value::create_undefined(ctx.native_context()),
        };
        JsContextImpl::call(ctx.native_context(), self.as_value(), this, argv)
    }

    fn call_raw<Args>(&self, this: Option<JsObject<'js, E>>, args: Args) -> E::Value
    where
        Args: IntoJsArgs<'js, E>,
    {
        let ctx = self.context();
        let argv = args.into_js_args(ctx.clone());
        self.call_with_argv(this, &argv)
    }

    pub fn new<F, P, K>(ctx: JsContext<'js, E>, f: F) -> JsResult<Self>
    where
        F: IntoJsCallable<E, P, K>,
        P: FromParams<E>,
        E: 'static,
    {
        RustFunc::new(f).into_js(ctx)
    }

    pub fn new_once<F, P, K>(ctx: JsContext<'js, E>, f: F) -> JsResult<Self>
    where
        F: IntoOnceJsCallable<E, P, K>,
        P: FromParams<E>,
        E: 'static,
    {
        RustFunc::new_once(f).into_js(ctx)
    }

    pub fn callback<F>(ctx: JsContext<'js, E>, arity: u32, callback: F) -> JsResult<Self>
    where
        F: for<'i> FnMut(
                JsContext<'i, E>,
                Option<JsObject<'i, E>>,
                Vec<JsValue<'i, E>>,
            ) -> JsResult<JsValue<'i, E>>
            + 'static,
        E: 'static,
    {
        RustFunc::new_callback(arity, callback).into_js(ctx)
    }

    pub fn callback_once<F>(ctx: JsContext<'js, E>, arity: u32, callback: F) -> JsResult<Self>
    where
        F: for<'i> FnOnce(
                JsContext<'i, E>,
                Option<JsObject<'i, E>>,
                Vec<JsValue<'i, E>>,
            ) -> JsResult<JsValue<'i, E>>
            + 'static,
        E: 'static,
    {
        RustFunc::new_callback_once(arity, callback).into_js(ctx)
    }

    pub fn call<Args, R>(&self, this: Option<JsObject<'js, E>>, args: Args) -> JsResult<R>
    where
        Args: IntoJsArgs<'js, E>,
        R: FromJsValue<'js, E>,
        E::Value: JsObjectOps + JsValueMapper<'js, E>,
    {
        let ctx = self.context();
        let result = self.call_raw(this, args);
        result.try_convert::<R>(ctx)
    }

    pub fn name(self, name: &str) -> JsResult<Self> {
        let ctx = self.0.context();
        let name_value = JsValue::from_rust(ctx, name);
        self.0.define_property(
            "name",
            PropertyDescriptor::from_value(name_value)
                .readonly()
                .hidden()
                .configurable(),
        )?;
        Ok(self)
    }

    pub(crate) fn into_value(self) -> E::Value {
        self.0.into_value()
    }
}

impl<E: JsEngine> RustFunc<E>
where
    E::Value: JsObjectOps + 'static,
{
    pub(crate) fn into_js<'js>(self, ctx: JsContext<'js, E>) -> JsResult<JsFunc<'js, E>>
    where
        E: 'static,
    {
        let length = self.parameter_required_count();
        let class = Class::lookup::<RustFunc<E>>(&ctx)?;
        let obj = class.instance(self);
        obj.define_property(
            "length",
            crate::PropertyDescriptor::from_rust(ctx, length as i32)
                .readonly()
                .hidden()
                .non_configurable(),
        )?;
        Ok(JsFunc(obj))
    }
}

impl<'js, E: JsEngine> JsParameterType for JsFunc<'js, E> {}
