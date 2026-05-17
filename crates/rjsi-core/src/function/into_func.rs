// Contains code from requickjs.
// https://github.com/DelSkayn/rquickjs/blob/master/core/src/value/function/into_func.rs

use super::IntoJsFunc;
use super::params::{FromParams, ParamRequirement, Params};
use super::types::{Func, MutFn, OnceFn, ThisState, ThisStateMut, WithCx};
use crate::{Context, Engine, Error, NativeState, NativeStateSupport, Result, ToJs, Value};

impl<E, F, R> IntoJsFunc<E, ()> for F
where
    E: Engine,
    F: Fn() -> R + 'static,
    R: for<'cx> ToJs<'cx, E>,
{
    fn param_requirements() -> ParamRequirement {
        ParamRequirement::none()
    }

    fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
        let mut acc = params.access();
        let r = (self)();
        r.to_js(acc.ctx())
    }
}

macro_rules! impl_into_js_func {
    ($($A:ident),+) => {
        #[allow(non_snake_case)]
        impl<E, F, R, $($A,)+> IntoJsFunc<E, ($($A,)+)> for F
        where
            E: Engine,
            F: Fn($($A),+) -> R + 'static,
            ($($A,)+): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)+) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)+) = <($($A,)+) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let r = (self)($($A),+);
                r.to_js(acc.ctx())
            }
        }

        #[allow(non_snake_case)]
        impl<E, F, R, $($A,)+> IntoJsFunc<E, ($($A,)+)> for MutFn<F>
        where
            E: Engine,
            F: FnMut($($A),+) -> R + 'static,
            ($($A,)+): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)+) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)+) = <($($A,)+) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let mut lock = self
                    .0
                    .try_borrow_mut()
                    .map_err(|_| Error::type_err("host function already borrowed mutably"))?;
                let r = (lock)($($A),+);
                r.to_js(acc.ctx())
            }
        }

        #[allow(non_snake_case)]
        impl<E, F, R, $($A,)+> IntoJsFunc<E, ($($A,)+)> for OnceFn<F>
        where
            E: Engine,
            F: FnOnce($($A),+) -> R + 'static,
            ($($A,)+): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)+) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)+) = <($($A,)+) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let f = self
                    .0
                    .take()
                    .ok_or_else(|| Error::type_err("once host function already consumed"))?;
                let r = (f)($($A),+);
                r.to_js(acc.ctx())
            }
        }

        #[allow(non_snake_case)]
        impl<E, F, R, $($A,)+> IntoJsFunc<E, WithCx<($($A,)+)>> for F
        where
            E: Engine,
            F: for<'cx> Fn(&mut Context<'cx, E>, $($A),+) -> R + 'static,
            ($($A,)+): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)+) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)+) = <($($A,)+) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let r = (self)(acc.ctx(), $($A),+);
                r.to_js(acc.ctx())
            }
        }

        #[allow(non_snake_case)]
        impl<E, F, R, $($A,)+> IntoJsFunc<E, WithCx<($($A,)+)>> for MutFn<F>
        where
            E: Engine,
            F: for<'cx> FnMut(&mut Context<'cx, E>, $($A),+) -> R + 'static,
            ($($A,)+): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)+) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)+) = <($($A,)+) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let mut lock = self
                    .0
                    .try_borrow_mut()
                    .map_err(|_| Error::type_err("host function already borrowed mutably"))?;
                let r = (lock)(acc.ctx(), $($A),+);
                r.to_js(acc.ctx())
            }
        }
    };
}

impl<E, F, R> IntoJsFunc<E, WithCx<()>> for F
where
    E: Engine,
    F: for<'cx> Fn(&mut Context<'cx, E>) -> R + 'static,
    R: for<'cx> ToJs<'cx, E>,
{
    fn param_requirements() -> ParamRequirement {
        ParamRequirement::none()
    }

    fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
        let mut acc = params.access();
        let r = (self)(acc.ctx());
        r.to_js(acc.ctx())
    }
}

impl<E, F, R> IntoJsFunc<E, WithCx<()>> for MutFn<F>
where
    E: Engine,
    F: for<'cx> FnMut(&mut Context<'cx, E>) -> R + 'static,
    R: for<'cx> ToJs<'cx, E>,
{
    fn param_requirements() -> ParamRequirement {
        ParamRequirement::none()
    }

    fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
        let mut acc = params.access();
        let mut lock = self
            .0
            .try_borrow_mut()
            .map_err(|_| Error::type_err("host function already borrowed mutably"))?;
        let r = (lock)(acc.ctx());
        r.to_js(acc.ctx())
    }
}

impl_into_js_func!(A1);
impl_into_js_func!(A1, A2);
impl_into_js_func!(A1, A2, A3);
impl_into_js_func!(A1, A2, A3, A4);
impl_into_js_func!(A1, A2, A3, A4, A5);
impl_into_js_func!(A1, A2, A3, A4, A5, A6);
impl_into_js_func!(A1, A2, A3, A4, A5, A6, A7);
impl_into_js_func!(A1, A2, A3, A4, A5, A6, A7, A8);

impl<E, F, P> IntoJsFunc<E, P> for Func<F, P>
where
    E: Engine,
    F: IntoJsFunc<E, P>,
    P: 'static,
{
    fn param_requirements() -> ParamRequirement {
        F::param_requirements()
    }

    fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
        self.0.call(params)
    }
}

macro_rules! impl_this_state_into_js_func {
    ($($A:ident),*) => {
        // `Fn(&mut S, A1, ...) -> R` via the `ThisStateMut<S, (A1, ...)>` marker.
        #[allow(non_snake_case)]
        impl<E, F, R, S $(, $A)*> IntoJsFunc<E, ThisStateMut<S, ($($A,)*)>> for F
        where
            E: Engine + NativeStateSupport,
            S: NativeState,
            F: for<'s> Fn(&'s mut S $(, $A)*) -> R + 'static,
            ($($A,)*): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)*) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)*) = <($($A,)*) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let this = acc.take_this()?;
                let mut obj = this
                    .as_object()
                    .ok_or_else(|| Error::type_err("`this` is not an object"))?;
                let state = E::object_get_state_mut::<S>(acc.ctx(), &mut obj).ok_or_else(|| {
                    Error::type_err("`this` is missing the expected native state")
                })?;
                let r = (self)(state $(, $A)*);
                r.to_js(acc.ctx())
            }
        }

        // `Fn(&S, A1, ...) -> R` via the `ThisState<S, (A1, ...)>` marker.
        #[allow(non_snake_case)]
        impl<E, F, R, S $(, $A)*> IntoJsFunc<E, ThisState<S, ($($A,)*)>> for F
        where
            E: Engine + NativeStateSupport,
            S: NativeState,
            F: for<'s> Fn(&'s S $(, $A)*) -> R + 'static,
            ($($A,)*): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)*) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)*) = <($($A,)*) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let this = acc.take_this()?;
                let obj = this
                    .as_object()
                    .ok_or_else(|| Error::type_err("`this` is not an object"))?;
                let state = E::object_get_state::<S>(acc.ctx(), &obj).ok_or_else(|| {
                    Error::type_err("`this` is missing the expected native state")
                })?;
                let r = (self)(state $(, $A)*);
                r.to_js(acc.ctx())
            }
        }

        // `Fn(&mut Context, &mut S, A1, ...) -> R` via `WithCx<ThisStateMut<S, (A1, ...)>>`.
        #[allow(non_snake_case)]
        impl<E, F, R, S $(, $A)*> IntoJsFunc<E, WithCx<ThisStateMut<S, ($($A,)*)>>> for F
        where
            E: Engine + NativeStateSupport,
            S: NativeState,
            F: for<'cx, 's> Fn(&mut Context<'cx, E>, &'s mut S $(, $A)*) -> R + 'static,
            ($($A,)*): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)*) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)*) = <($($A,)*) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let this = acc.take_this()?;
                let mut obj = this
                    .as_object()
                    .ok_or_else(|| Error::type_err("`this` is not an object"))?;
                let state = E::object_get_state_mut::<S>(acc.ctx(), &mut obj).ok_or_else(|| {
                    Error::type_err("`this` is missing the expected native state")
                })?;
                let r = (self)(acc.ctx(), state $(, $A)*);
                r.to_js(acc.ctx())
            }
        }

        // `Fn(&mut Context, &S, A1, ...) -> R` via `WithCx<ThisState<S, (A1, ...)>>`.
        #[allow(non_snake_case)]
        impl<E, F, R, S $(, $A)*> IntoJsFunc<E, WithCx<ThisState<S, ($($A,)*)>>> for F
        where
            E: Engine + NativeStateSupport,
            S: NativeState,
            F: for<'cx, 's> Fn(&mut Context<'cx, E>, &'s S $(, $A)*) -> R + 'static,
            ($($A,)*): for<'cx> FromParams<'cx, E>,
            R: for<'cx> ToJs<'cx, E>,
        {
            fn param_requirements() -> ParamRequirement {
                <($($A,)*) as FromParams<'static, E>>::param_requirements()
            }

            fn call<'a, 'cx>(&self, params: Params<'a, 'cx, E>) -> Result<Value<'cx, E>> {
                let mut acc = params.access();
                let ($($A,)*) = <($($A,)*) as FromParams<'cx, E>>::from_params(&mut acc)?;
                let this = acc.take_this()?;
                let obj = this
                    .as_object()
                    .ok_or_else(|| Error::type_err("`this` is not an object"))?;
                let state = E::object_get_state::<S>(acc.ctx(), &obj).ok_or_else(|| {
                    Error::type_err("`this` is missing the expected native state")
                })?;
                let r = (self)(acc.ctx(), state $(, $A)*);
                r.to_js(acc.ctx())
            }
        }
    };
}

impl_this_state_into_js_func!();
impl_this_state_into_js_func!(A1);
impl_this_state_into_js_func!(A1, A2);
impl_this_state_into_js_func!(A1, A2, A3);
impl_this_state_into_js_func!(A1, A2, A3, A4);
impl_this_state_into_js_func!(A1, A2, A3, A4, A5);
impl_this_state_into_js_func!(A1, A2, A3, A4, A5, A6);
impl_this_state_into_js_func!(A1, A2, A3, A4, A5, A6, A7);
