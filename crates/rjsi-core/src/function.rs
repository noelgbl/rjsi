// Contains code from requickjs.
// https://github.com/DelSkayn/rquickjs/blob/master/core/src/value/function.rs

mod into_func;
mod params;
mod types;

use std::marker::PhantomData;

pub use params::{FromParam, FromParams, ParamRequirement, Params, ParamsAccessor};
pub use types::{
    Exhaustive, Flat, Func, MutFn, OnceFn, Opt, Rest, This, ThisState, ThisStateMut, WithCx
};

use crate::{Args, Context, Engine, Object, RawHostFn, Result, Value};

pub trait IntoJsFunc<E: Engine, P>: 'static {
    fn param_requirements() -> ParamRequirement;

    fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>>;
}

#[repr(transparent)]
pub struct Function<'cx, E: Engine> {
    pub(crate) raw: E::Function<'cx>,
}

impl<'cx, E: Engine> Clone for Function<'cx, E>
where
    E::Function<'cx>: Clone,
{
    fn clone(&self) -> Self {
        Self { raw: self.raw.clone() }
    }
}

impl<'cx, E: Engine> Function<'cx, E> {
    pub fn new(raw: E::Function<'cx>) -> Self {
        Self { raw }
    }

    pub fn into_raw(self) -> E::Function<'cx> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Function<'cx> {
        &self.raw
    }

    pub fn call(
        &self,
        cx: &mut Context<'cx, E>,
        this: Value<'cx, E>,
        args: &[Value<'cx, E>],
    ) -> Result<Value<'cx, E>> {
        let raw_args: &[E::Value<'cx>] = unsafe {
            std::slice::from_raw_parts(args.as_ptr() as *const E::Value<'cx>, args.len())
        };

        E::function_call(&mut cx.raw, &self.raw, this.raw, raw_args).map(Value::new)
    }

    pub fn call_no_args(&self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        let this: Value<'cx, E> = Value::new(E::make_undefined(&mut cx.raw));
        E::function_call(&mut cx.raw, &self.raw, this.raw, &[]).map(Value::new)
    }

    pub fn into_value(self) -> Value<'cx, E> {
        Value::new(E::function_to_value(self.raw))
    }

    pub fn into_object(self) -> Object<'cx, E> {
        Object::new(E::function_to_object(self.raw))
    }
}

impl<'cx, E: Engine> crate::convert::ToJs<'cx, E> for Function<'cx, E> {
    fn to_js(self, _cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        Ok(self.into_value())
    }
}

pub(crate) struct IntoJsFuncAdapter<F, P> {
    func: F,
    req: ParamRequirement,
    _p: PhantomData<fn() -> P>,
}

impl<F, P> IntoJsFuncAdapter<F, P> {
    pub(crate) fn new<E: Engine>(func: F) -> Self
    where
        F: IntoJsFunc<E, P>,
    {
        Self {
            req: F::param_requirements(),
            func,
            _p: PhantomData,
        }
    }
}

impl<E, F, P> RawHostFn<E> for IntoJsFuncAdapter<F, P>
where
    E: Engine,
    F: IntoJsFunc<E, P>,
    P: 'static,
{
    fn call<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: Args<'rt, E>,
    ) -> Result<Value<'rt, E>> {
        let params = Params::new(cx, this, &args);
        params.check_params(self.req)?;
        self.func.call(params)
    }
}
