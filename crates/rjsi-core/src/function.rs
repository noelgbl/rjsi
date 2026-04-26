use std::cell::RefCell;
use std::marker::PhantomData;

use crate::{HostError, JsEngine, JsResult};

/// Engine-provided view over host callback arguments.
///
/// Implementations may borrow directly from the native engine callback frame.
/// Returning `E::Value<'js>` must stay scope-local; it must not implicitly root
/// or persist values. `SliceHostArgs` is only a convenience implementation for
/// tests and simple engines whose local values are cheap to copy or duplicate.
pub trait HostArgs<'a, 'js, E: JsEngine>
where
    'js: 'a,
{
    fn len(&self) -> usize;
    fn this(&self, scope: &mut E::Scope<'js>) -> Option<E::Value<'js>>;
    fn get(&self, scope: &mut E::Scope<'js>, index: usize) -> Option<E::Value<'js>>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct SliceHostArgs<'a, 'js, E: JsEngine> {
    this: Option<&'a E::Value<'js>>,
    args: &'a [E::Value<'js>],
}

impl<'a, 'js, E: JsEngine> SliceHostArgs<'a, 'js, E> {
    pub fn new(this: Option<&'a E::Value<'js>>, args: &'a [E::Value<'js>]) -> Self {
        Self { this, args }
    }
}

impl<'a, 'js, E> HostArgs<'a, 'js, E> for SliceHostArgs<'a, 'js, E>
where
    E: JsEngine,
    E::Value<'js>: Clone,
    'js: 'a,
{
    fn len(&self) -> usize {
        self.args.len()
    }

    fn this(&self, _scope: &mut E::Scope<'js>) -> Option<E::Value<'js>> {
        self.this.cloned()
    }

    fn get(&self, _scope: &mut E::Scope<'js>, index: usize) -> Option<E::Value<'js>> {
        self.args.get(index).cloned()
    }
}

pub struct ArrayHostArgs<'js, E: JsEngine, const N: usize> {
    this: Option<E::Value<'js>>,
    args: [E::Value<'js>; N],
}

impl<'js, E: JsEngine, const N: usize> ArrayHostArgs<'js, E, N> {
    pub fn new(this: Option<E::Value<'js>>, args: [E::Value<'js>; N]) -> Self {
        Self { this, args }
    }
}

impl<'a, 'js, E, const N: usize> HostArgs<'a, 'js, E> for ArrayHostArgs<'js, E, N>
where
    E: JsEngine,
    E::Value<'js>: Clone,
    'js: 'a,
{
    fn len(&self) -> usize {
        N
    }

    fn this(&self, _scope: &mut E::Scope<'js>) -> Option<E::Value<'js>> {
        self.this.clone()
    }

    fn get(&self, _scope: &mut E::Scope<'js>, index: usize) -> Option<E::Value<'js>> {
        self.args.get(index).cloned()
    }
}

pub struct ParamsAccessor<'a, 'js, E: JsEngine>
where
    'js: 'a,
{
    scope: *mut E::Scope<'js>,
    args: E::HostArgs<'a, 'js>,
    next_arg: usize,
    marker: PhantomData<&'a mut E::Scope<'js>>,
}

impl<'a, 'js, E: JsEngine> ParamsAccessor<'a, 'js, E>
where
    'js: 'a,
{
    pub fn new(scope: &'a mut E::Scope<'js>, args: E::HostArgs<'a, 'js>) -> Self {
        Self {
            scope,
            args,
            next_arg: 0,
            marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.args.len().saturating_sub(self.next_arg)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn scope(&mut self) -> &mut E::Scope<'js> {
        // SAFETY: ParamsAccessor is constructed from a unique scope borrow and
        // does not expose another mutable scope reference at the same time.
        unsafe { &mut *self.scope }
    }

    pub fn this(&mut self) -> Option<E::Value<'js>> {
        let scope = self.scope;
        let args = &self.args;
        // SAFETY: the raw pointer was created from a unique scope borrow.
        args.this(unsafe { &mut *scope })
    }

    pub fn arg(&mut self, index: usize) -> Option<E::Value<'js>> {
        let absolute = self.next_arg + index;
        let scope = self.scope;
        let args = &self.args;
        // SAFETY: the raw pointer was created from a unique scope borrow.
        args.get(unsafe { &mut *scope }, absolute)
    }

    pub fn next_arg(&mut self) -> Option<E::Value<'js>> {
        let value = self.arg(0);
        if value.is_some() {
            self.next_arg += 1;
        }
        value
    }
}

pub trait HostFunction<E: JsEngine>: 'static {
    fn required_params(&self) -> usize {
        0
    }

    fn call<'a, 'js>(
        &mut self,
        accessor: &mut ParamsAccessor<'a, 'js, E>,
    ) -> JsResult<E::Value<'js>>
    where
        'js: 'a;
}

pub trait IntoJs<'js, S: crate::JsScope<'js>> {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>>;
}

pub trait FromJs<'js, S: crate::JsScope<'js>>: Sized {
    fn from_js(scope: &mut S, value: &<S::Engine as JsEngine>::Value<'js>) -> JsResult<Self>;
}

impl<'js, S: crate::JsScope<'js>> IntoJs<'js, S> for () {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>> {
        Ok(scope.undefined())
    }
}

impl<'js, S: crate::JsScope<'js>> IntoJs<'js, S> for bool {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>> {
        Ok(scope.boolean(self))
    }
}

impl<'js, S: crate::JsScope<'js>> IntoJs<'js, S> for f64 {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>> {
        Ok(scope.number(self))
    }
}

impl<'js, S: crate::JsScope<'js>> IntoJs<'js, S> for i32 {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>> {
        Ok(scope.number(self as f64))
    }
}

impl<'js, S: crate::JsScope<'js>> IntoJs<'js, S> for &str {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>> {
        Ok(scope.string(self))
    }
}

impl<'js, S: crate::JsScope<'js>> IntoJs<'js, S> for String {
    fn into_js(self, scope: &mut S) -> JsResult<<S::Engine as JsEngine>::Value<'js>> {
        Ok(scope.string(&self))
    }
}

impl<'js, S: crate::JsScope<'js>> FromJs<'js, S> for bool {
    fn from_js(scope: &mut S, value: &<S::Engine as JsEngine>::Value<'js>) -> JsResult<Self> {
        scope
            .to_boolean(value)
            .ok_or_else(|| HostError::type_error(crate::error::E_TYPE, "expected boolean").into())
    }
}

impl<'js, S: crate::JsScope<'js>> FromJs<'js, S> for f64 {
    fn from_js(scope: &mut S, value: &<S::Engine as JsEngine>::Value<'js>) -> JsResult<Self> {
        scope
            .to_number(value)
            .ok_or_else(|| HostError::type_error(crate::error::E_TYPE, "expected number").into())
    }
}

impl<'js, S: crate::JsScope<'js>> FromJs<'js, S> for i32 {
    fn from_js(scope: &mut S, value: &<S::Engine as JsEngine>::Value<'js>) -> JsResult<Self> {
        let value = scope
            .to_number(value)
            .ok_or_else(|| HostError::type_error(crate::error::E_TYPE, "expected number"))?;
        Ok(value as i32)
    }
}

impl<'js, S: crate::JsScope<'js>> FromJs<'js, S> for String {
    fn from_js(scope: &mut S, value: &<S::Engine as JsEngine>::Value<'js>) -> JsResult<Self> {
        scope
            .to_string(value)
            .ok_or_else(|| HostError::type_error(crate::error::E_TYPE, "expected string").into())
    }
}

pub trait FromHostArgs<'a, 'js, E: JsEngine>: Sized
where
    'js: 'a,
{
    fn from_host_args(accessor: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<Self>;
}

impl<'a, 'js, E: JsEngine> FromHostArgs<'a, 'js, E> for ()
where
    'js: 'a,
{
    fn from_host_args(_accessor: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<Self> {
        Ok(())
    }
}

impl<'a, 'js, E, A> FromHostArgs<'a, 'js, E> for (A,)
where
    E: JsEngine,
    E::Scope<'js>: crate::JsScope<'js, Engine = E>,
    A: FromJs<'js, E::Scope<'js>>,
    'js: 'a,
{
    fn from_host_args(accessor: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<Self> {
        let value = accessor
            .next_arg()
            .ok_or_else(|| HostError::invalid_arg_count(1, 0))?;
        let scope = accessor.scope();
        Ok((A::from_js(scope, &value)?,))
    }
}

impl<'a, 'js, E, A, B> FromHostArgs<'a, 'js, E> for (A, B)
where
    E: JsEngine,
    E::Scope<'js>: crate::JsScope<'js, Engine = E>,
    A: FromJs<'js, E::Scope<'js>>,
    B: FromJs<'js, E::Scope<'js>>,
    'js: 'a,
{
    fn from_host_args(accessor: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<Self> {
        let first = accessor
            .next_arg()
            .ok_or_else(|| HostError::invalid_arg_count(2, 0))?;
        let second = accessor
            .next_arg()
            .ok_or_else(|| HostError::invalid_arg_count(2, 1))?;
        let scope = accessor.scope();
        Ok((A::from_js(scope, &first)?, B::from_js(scope, &second)?))
    }
}

pub trait IntoHostReturn<'js, E: JsEngine> {
    fn into_host_return(self, scope: &mut E::Scope<'js>) -> JsResult<E::Value<'js>>;
}

impl<'js, E, T> IntoHostReturn<'js, E> for T
where
    E: JsEngine,
    E::Scope<'js>: crate::JsScope<'js, Engine = E>,
    T: IntoJs<'js, E::Scope<'js>>,
{
    fn into_host_return(self, scope: &mut E::Scope<'js>) -> JsResult<E::Value<'js>> {
        self.into_js(scope)
    }
}

pub struct TypedHostFunction<F, Args> {
    function: F,
    marker: PhantomData<fn() -> Args>,
}

impl<F, Args> TypedHostFunction<F, Args> {
    pub fn new(function: F) -> Self {
        Self {
            function,
            marker: PhantomData,
        }
    }
}

impl<E, F, R> HostFunction<E> for TypedHostFunction<F, ()>
where
    E: JsEngine,
    F: for<'js> FnMut(&mut E::Scope<'js>) -> R + 'static,
    R: for<'js> IntoHostReturn<'js, E>,
{
    fn call<'a, 'js>(
        &mut self,
        accessor: &mut ParamsAccessor<'a, 'js, E>,
    ) -> JsResult<E::Value<'js>>
    where
        'js: 'a,
    {
        (self.function)(accessor.scope()).into_host_return(accessor.scope())
    }
}

impl<E, F, A: 'static, R> HostFunction<E> for TypedHostFunction<F, (A,)>
where
    E: JsEngine,
    E::Scope<'static>: Sized,
    F: for<'js> FnMut(A) -> R + 'static,
    for<'a, 'js> (A,): FromHostArgs<'a, 'js, E>,
    R: for<'js> IntoHostReturn<'js, E>,
{
    fn call<'a, 'js>(
        &mut self,
        accessor: &mut ParamsAccessor<'a, 'js, E>,
    ) -> JsResult<E::Value<'js>>
    where
        'js: 'a,
    {
        let (arg,) = <(A,)>::from_host_args(accessor)?;
        (self.function)(arg).into_host_return(accessor.scope())
    }
}

impl<E, F> HostFunction<E> for F
where
    E: JsEngine,
    F: for<'a, 'js> FnMut(&mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>> + 'static,
{
    fn call<'a, 'js>(
        &mut self,
        accessor: &mut ParamsAccessor<'a, 'js, E>,
    ) -> JsResult<E::Value<'js>>
    where
        'js: 'a,
    {
        self(accessor)
    }
}

type HostAccessorCallback<E> = dyn for<'a, 'js> FnMut(&mut ParamsAccessor<'a, 'js, E>) -> JsResult<<E as JsEngine>::Value<'js>>
    + 'static;

pub struct RustFunc<E: JsEngine> {
    required_params: usize,
    callback: RefCell<Box<HostAccessorCallback<E>>>,
}

impl<E: JsEngine> RustFunc<E> {
    pub fn new(
        required_params: usize,
        callback: impl for<'a, 'js> FnMut(&mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
        + 'static,
    ) -> Self {
        Self {
            required_params,
            callback: RefCell::new(Box::new(callback)),
        }
    }

    pub fn call<'a, 'js>(
        &self,
        accessor: &mut ParamsAccessor<'a, 'js, E>,
    ) -> JsResult<E::Value<'js>>
    where
        'js: 'a,
    {
        let got = accessor.len();
        if got < self.required_params {
            return Err(HostError::invalid_arg_count(self.required_params, got).into());
        }
        (self.callback.borrow_mut())(accessor)
    }
}
