use crate::{
    HostError, IntoJsValue, JsClass, JsContext, JsEngine, JsObject, JsResult, JsTypeOf, JsValue,
};
use std::cell::RefCell;

mod parameter;
pub use parameter::{
    FromParams, JsClassRef, JsParameterType, Optional, ParamsAccessor, Rest, This, ThisMut,
};

pub type HostCallback<E> = dyn for<'js> FnMut(
        JsContext<'js, E>,
        Option<JsObject<'js, E>>,
        Vec<JsValue<'js, E>>,
    ) -> JsResult<JsValue<'js, E>>
    + 'static;

pub type HostCallbackOnce<E> = dyn for<'js> FnOnce(
        JsContext<'js, E>,
        Option<JsObject<'js, E>>,
        Vec<JsValue<'js, E>>,
    ) -> JsResult<JsValue<'js, E>>
    + 'static;

/// Low-level host callback that reads arguments through a borrowed accessor.
///
/// Unlike [`HostCallback`], this path does not materialize a `Vec<JsValue>`.
pub type HostAccessorCallback<E> =
    dyn for<'js> FnMut(&mut ParamsAccessor<'js, E>) -> JsResult<<E as JsEngine>::Value> + 'static;

/// Container to hold rust closure/function that's callable from JS.
pub struct RustFunc<E: JsEngine> {
    func: JsCallable<E>,
    required_params: u32,
}

type FnMutClosure<E> =
    dyn for<'js> FnMut(&mut ParamsAccessor<'js, E>) -> JsResult<<E as JsEngine>::Value>;
type FnOnceClosure<E> =
    dyn for<'js> FnOnce(&mut ParamsAccessor<'js, E>) -> JsResult<<E as JsEngine>::Value>;

pub enum JsCallable<E: JsEngine> {
    FnMut(RefCell<Box<FnMutClosure<E>>>),
    FnOnce(RefCell<Option<Box<FnOnceClosure<E>>>>),
    Accessor(RefCell<Box<HostAccessorCallback<E>>>),
    Callback(RefCell<Box<HostCallback<E>>>),
    CallbackOnce(RefCell<Option<Box<HostCallbackOnce<E>>>>),
}

#[doc(hidden)]
pub trait IntoJsCallable<E: JsEngine, P, K> {
    fn into_js_callable(self) -> JsCallable<E>;
}

#[doc(hidden)]
pub trait IntoOnceJsCallable<E: JsEngine, P, K> {
    fn into_js_callable(self) -> JsCallable<E>;
}

#[doc(hidden)]
pub struct KFnMut;
#[doc(hidden)]
pub struct KFnOnce;

impl<E: JsEngine> RustFunc<E> {
    pub(crate) fn new<F, P, K>(f: F) -> Self
    where
        F: IntoJsCallable<E, P, K>,
        P: FromParams<E>,
    {
        let required_params = P::param_requirements().required_count() as u32;
        Self {
            func: f.into_js_callable(),
            required_params,
        }
    }

    pub(crate) fn new_once<F, P, K>(f: F) -> Self
    where
        F: IntoOnceJsCallable<E, P, K>,
        P: FromParams<E>,
    {
        let required_params = P::param_requirements().required_count() as u32;
        Self {
            func: f.into_js_callable(),
            required_params,
        }
    }

    pub(crate) fn new_callback<F>(arity: u32, callback: F) -> Self
    where
        F: for<'js> FnMut(
                JsContext<'js, E>,
                Option<JsObject<'js, E>>,
                Vec<JsValue<'js, E>>,
            ) -> JsResult<JsValue<'js, E>>
            + 'static,
    {
        let boxed: Box<HostCallback<E>> = Box::new(callback);
        Self {
            func: JsCallable::Callback(RefCell::new(boxed)),
            required_params: arity,
        }
    }

    pub(crate) fn new_accessor_callback<F>(arity: u32, callback: F) -> Self
    where
        F: for<'js> FnMut(&mut ParamsAccessor<'js, E>) -> JsResult<E::Value> + 'static,
    {
        let boxed: Box<HostAccessorCallback<E>> = Box::new(callback);
        Self {
            func: JsCallable::Accessor(RefCell::new(boxed)),
            required_params: arity,
        }
    }

    pub(crate) fn new_callback_once<F>(arity: u32, callback: F) -> Self
    where
        F: for<'js> FnOnce(
                JsContext<'js, E>,
                Option<JsObject<'js, E>>,
                Vec<JsValue<'js, E>>,
            ) -> JsResult<JsValue<'js, E>>
            + 'static,
    {
        let boxed: Box<HostCallbackOnce<E>> = Box::new(callback);
        Self {
            func: JsCallable::CallbackOnce(RefCell::new(Some(boxed))),
            required_params: arity,
        }
    }

    pub fn call<'js>(&mut self, accessor: &mut ParamsAccessor<'js, E>) -> JsResult<E::Value>
    where
        E::Value: JsTypeOf,
    {
        let num_args = accessor.args_len() as u32;
        if num_args < self.required_params {
            return Err(HostError::invalid_arg_count(self.required_params, num_args).into());
        }

        match &self.func {
            JsCallable::FnMut(f) => f.borrow_mut()(accessor),
            JsCallable::FnOnce(f) => f.take().ok_or_else(HostError::once_fn_called)?(accessor),
            JsCallable::Accessor(callback) => callback.borrow_mut()(accessor),
            JsCallable::Callback(callback) => {
                let (ctx, this, args) = accessor.raw_parts();
                callback.borrow_mut()(ctx, this, args).map(|v| v.into_inner())
            }
            JsCallable::CallbackOnce(callback) => {
                let (ctx, this, args) = accessor.raw_parts();
                callback.take().ok_or_else(HostError::once_fn_called)?(ctx, this, args)
                    .map(|v| v.into_inner())
            }
        }
    }

    pub(crate) fn parameter_required_count(&self) -> u32 {
        self.required_params
    }
}

pub struct Constructor<E: JsEngine>(pub(crate) RustFunc<E>);

impl<E: JsEngine> Constructor<E> {
    pub fn new<F, P, K>(f: F) -> Self
    where
        F: IntoJsCallable<E, P, K>,
        P: FromParams<E>,
    {
        Self(RustFunc::new(f))
    }

    pub fn callback<F>(arity: u32, callback: F) -> Self
    where
        F: for<'js> FnMut(
                JsContext<'js, E>,
                Option<JsObject<'js, E>>,
                Vec<JsValue<'js, E>>,
            ) -> JsResult<JsValue<'js, E>>
            + 'static,
    {
        Self(RustFunc::new_callback(arity, callback))
    }

    pub fn accessor_callback<F>(arity: u32, callback: F) -> Self
    where
        F: for<'js> FnMut(&mut ParamsAccessor<'js, E>) -> JsResult<E::Value> + 'static,
    {
        Self(RustFunc::new_accessor_callback(arity, callback))
    }

    pub fn call<'js>(&mut self, accessor: &mut ParamsAccessor<'js, E>) -> JsResult<E::Value>
    where
        E::Value: JsTypeOf,
    {
        self.0.call(accessor)
    }
}

impl<E> JsClass<E> for RustFunc<E>
where
    E: JsEngine + 'static,
    E::Value: crate::JsObjectOps,
{
    const NAME: &'static str = "RustFunc";
    const CALLABLE: bool = true;

    fn data_constructor() -> Constructor<E> {
        panic!("Never 'new RustFunc()' in JS");
    }

    fn class_setup(class: &crate::ClassSetup<'_, '_, E>) -> JsResult<()> {
        let fn_proto = class
            .context()
            .clone()
            .global()?
            .get::<_, crate::JsObject<'_, E>>("Function")?
            .get::<_, crate::JsObject<'_, E>>("prototype")?;
        class.prototype_object().prototype(fn_proto);
        Ok(())
    }
}

macro_rules! impl_js_callable_func {
    ($($t:ident),* $(,)?) => {
        impl<E, R, Fun $(,$t)*> IntoJsCallable<E, ($($t,)*), KFnMut> for Fun
        where
            Fun: FnMut($($t),*) -> R + 'static,
            E: JsEngine + 'static,
            ($($t,)*): FromParams<E>,
            R: for<'js> IntoJsValue<'js, E>
        {
            fn into_js_callable(self) -> JsCallable<E> {
                let mut f = self;
                let closure =
                    move |accessor: &mut ParamsAccessor<'_, E>| {
                        let params = <($($t,)*)>::from_params(accessor)?;
                        #[allow(non_snake_case)]
                        let ($($t,)*) = params;
                        let result = f($($t),*);
                        let ctx = accessor.context();
                        Ok(result.into_js_value(ctx).into_inner())
                    };
                JsCallable::FnMut(RefCell::new(Box::new(closure)))
            }
        }
    };
}

macro_rules! impl_js_oncecallable_func {
    ($($t:ident),* $(,)?) => {
        impl<E, R, Fun $(,$t)*> IntoOnceJsCallable<E, ($($t,)*), KFnOnce> for Fun
        where
            Fun: FnOnce($($t),*) -> R + 'static,
            E: JsEngine + 'static,
            ($($t,)*): FromParams<E>,
            R: for<'js> IntoJsValue<'js, E>
        {
            fn into_js_callable(self) -> JsCallable<E> {
                let f = self;
                let closure =
                    move |accessor: &mut ParamsAccessor<'_, E>| {
                        let params = <($($t,)*)>::from_params(accessor)?;
                        #[allow(non_snake_case)]
                        let ($($t,)*) = params;
                        let result = f($($t),*);
                        let ctx = accessor.context();
                        Ok(result.into_js_value(ctx).into_inner())
                    };
                JsCallable::FnOnce(RefCell::new(Some(Box::new(closure))))
            }
        }
    };
}

impl_js_callable_func!();
impl_js_callable_func!(P1);
impl_js_callable_func!(P1, P2);
impl_js_callable_func!(P1, P2, P3);
impl_js_callable_func!(P1, P2, P3, P4);
impl_js_callable_func!(P1, P2, P3, P4, P5);
impl_js_callable_func!(P1, P2, P3, P4, P5, P6);
impl_js_callable_func!(P1, P2, P3, P4, P5, P6, P7);
impl_js_callable_func!(P1, P2, P3, P4, P5, P6, P7, P8);

impl_js_oncecallable_func!();
impl_js_oncecallable_func!(P1);
impl_js_oncecallable_func!(P1, P2);
impl_js_oncecallable_func!(P1, P2, P3);
impl_js_oncecallable_func!(P1, P2, P3, P4);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5, P6);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5, P6, P7);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5, P6, P7, P8);

#[cfg(test)]
mod tests {}
