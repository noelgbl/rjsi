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

    fn call<'a, 'js>(&self, params: Params<'a, 'js, E>) -> Result<Value<'js, E>>;
}

#[repr(transparent)]
pub struct Function<'js, E: Engine> {
    pub(crate) raw: E::Function<'js>,
    _inv: PhantomData<crate::markers::Invariant<'js>>,
}

impl<'js, E: Engine> Clone for Function<'js, E>
where
    E::Function<'js>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _inv: PhantomData,
        }
    }
}

impl<'js, E: Engine> Function<'js, E> {
    pub fn new(raw: E::Function<'js>) -> Self {
        Self {
            raw,
            _inv: PhantomData,
        }
    }

    pub fn into_raw(self) -> E::Function<'js> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Function<'js> {
        &self.raw
    }

    pub fn call(
        &self,
        cx: &mut Context<'js, E>,
        this: Value<'js, E>,
        args: &[Value<'js, E>],
    ) -> Result<Value<'js, E>> {
        let raw_args: &[E::Value<'js>] = unsafe {
            std::slice::from_raw_parts(args.as_ptr() as *const E::Value<'js>, args.len())
        };

        E::function_call(&mut cx.raw, &self.raw, this.raw, raw_args).map(Value::new)
    }

    pub fn call_no_args(&self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        let this: Value<'js, E> = Value::new(E::make_undefined(&mut cx.raw));
        E::function_call(&mut cx.raw, &self.raw, this.raw, &[]).map(Value::new)
    }

    pub fn into_value(self) -> Value<'js, E> {
        Value::new(E::function_to_value(self.raw))
    }

    pub fn into_object(self) -> Object<'js, E> {
        Object::new(E::function_to_object(self.raw))
    }
}

impl<'js, E: Engine> crate::convert::ToJs<'js, E> for Function<'js, E> {
    fn to_js(self, _cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
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
    fn call<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
        this: Value<'js, E>,
        args: Args<'js, E>,
    ) -> Result<Value<'js, E>> {
        let params = Params::new(cx, this, &args);
        params.check_params(self.req)?;
        self.func.call(params)
    }
}
