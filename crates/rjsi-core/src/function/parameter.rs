use crate::{
    FromJsValue, JsArrayOps, JsClass, JsContext, JsEngine, JsObject, JsObjectOps, JsResult,
    JsTypeOf, JsValue,
};
use std::cell::{Ref, RefMut};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::Deref;

/// Arguments retrieved from the JavaScript side for calling Rust functions.
pub struct ParamsAccessor<'js, E: JsEngine> {
    ctx: JsContext<'js, E>,
    this: E::Value,
    args: VecDeque<E::Value>,
    is_last_param: bool,
}

impl<'js, E: JsEngine> ParamsAccessor<'js, E> {
    pub fn new(ctx: JsContext<'js, E>, this: E::Value, args: Vec<E::Value>) -> Self {
        Self {
            ctx,
            this,
            args: args.into(),
            is_last_param: false,
        }
    }

    fn set_last_param(&mut self, is_last: bool) {
        self.is_last_param = is_last;
    }

    fn next_arg(&mut self) -> Option<E::Value> {
        self.args.pop_front()
    }

    pub fn get_this(&self) -> E::Value {
        self.this.clone()
    }

    pub(crate) fn context(&self) -> JsContext<'js, E> {
        self.ctx.clone()
    }

    pub(crate) fn raw_parts(
        &mut self,
    ) -> (
        JsContext<'js, E>,
        Option<JsObject<'js, E>>,
        Vec<JsValue<'js, E>>,
    )
    where
        E::Value: JsTypeOf,
    {
        let ctx = self.ctx.clone();
        let this = if self.this.is_undefined() {
            None
        } else {
            Some(
                JsObject::from_js_value(
                    ctx.clone(),
                    JsValue::from_raw(ctx.clone(), self.this.clone()),
                )
                .unwrap(),
            )
        };
        let args = self
            .args
            .drain(..)
            .map(|value| JsValue::from_raw(ctx.clone(), value))
            .collect();
        (ctx, this, args)
    }

    // length changed since its content will be removed
    pub(crate) fn args_len(&self) -> usize {
        self.args.len()
    }
}

/// Represents the `this` context in JavaScript function calls.
///
/// # Usage
/// - Used to capture the JavaScript `this` context in Rust functions
/// - Must be the first parameter if present
/// - Does not count towards required parameter count
///
/// # Example
/// ```ignore
/// use rjsi_core::function::parameter::This;
///
/// fn method(this: This<MyStruct>, x: i32) {
///     let my_struct: &MyStruct = &this;
/// }
/// ```
pub struct This<T>(pub T);

/// Represents the `this` context in JavaScript function calls with mutable access
pub struct ThisMut<'js, T, E: JsEngine>(pub(crate) JsObject<'js, E>, PhantomData<T>);

/// A non-owning handle to a JS class instance.
///
/// Unlike `FromJsValue<T>` for clone-enabled classes, this keeps the original JS
/// object identity and lets Rust borrow the underlying class data on demand.
pub struct JsClassRef<'js, T, E: JsEngine>(JsObject<'js, E>, PhantomData<T>);

/// Represents an optional parameter in JavaScript function calls.
///
/// # Usage
/// - Used for parameters that may or may not be provided
/// - Wraps the parameter type in `Option<T>`
/// - Does not count towards required parameter count
/// - Can appear anywhere in the parameter list
///
/// # Example
/// ```ignore
/// use rjsi_core::function::parameter::Optional;
///
/// fn func(x: i32, opt: Optional<String>) {
///     // Access the optional value via deref
///     if let Some(s) = &*opt {
///         println!("Optional param provided: {}", s);
///     }
/// }
/// ```
pub struct Optional<T>(pub Option<T>);

/// Represents rest parameters in JavaScript function calls.
///
/// # Usage
/// - Collects all remaining arguments into a `Vec<T>`
/// - Must be the last parameter if present
/// - Does not count towards required parameter count
/// - Useful for variadic functions
///
/// # Example
/// ```ignore
/// use rjsi_core::function::parameter::Rest;
///
/// fn variadic(x: i32, rest: Rest<String>) {
///     // Access the rest parameters via deref
///     for s in &*rest {
///         println!("Rest param: {}", s);
///     }
/// }
/// ```
pub struct Rest<T>(pub Vec<T>);

impl<T> Deref for This<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, T, E> ThisMut<'js, T, E>
where
    E::Value: JsObjectOps,
    E: JsEngine,
    T: JsClass<E>,
{
    pub fn object(&self) -> JsObject<'js, E> {
        self.0.clone()
    }

    pub fn borrow_mut(&self) -> JsResult<RefMut<'_, T>> {
        self.0.borrow_mut::<T>()
    }
}

impl<'js, T, E> JsClassRef<'js, T, E>
where
    E::Value: JsObjectOps,
    E: JsEngine,
    T: JsClass<E>,
{
    pub fn object(&self) -> JsObject<'js, E> {
        self.0.clone()
    }

    pub fn borrow(&self) -> JsResult<Ref<'_, T>> {
        self.0.borrow::<T>()
    }

    pub fn borrow_mut(&self) -> JsResult<RefMut<'_, T>> {
        self.0.borrow_mut::<T>()
    }
}

impl<'js, T, E> Clone for JsClassRef<'js, T, E>
where
    E: JsEngine,
    E::Value: JsObjectOps,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<'js, T, E> Deref for JsClassRef<'js, T, E>
where
    E: JsEngine,
{
    type Target = JsObject<'js, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Deref for Optional<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Deref for Rest<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents parameter requirements for a function
/// - required_count: number of mandatory parameters
/// - exhaustive: if true, no extra parameters are allowed beyond the required ones
pub trait FromParams<E: JsEngine>: Sized {
    fn from_params<'js>(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self>;
    fn param_requirements() -> ParamRequirement;
}

pub struct ParamRequirement {
    required_count: usize,
    exhaustive: bool,
}

impl ParamRequirement {
    pub fn required_count(&self) -> usize {
        self.required_count
    }

    const fn single() -> Self {
        Self {
            required_count: 1,
            exhaustive: true,
        }
    }

    const fn optional() -> Self {
        Self {
            required_count: 0,
            exhaustive: false,
        }
    }

    const fn any() -> Self {
        Self {
            required_count: 0,
            exhaustive: false,
        }
    }
}

pub trait ParameterKind {
    fn param_requirement() -> ParamRequirement;
}

pub struct Regular<T>(PhantomData<T>);
impl<T> ParameterKind for Regular<T> {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::single()
    }
}

pub struct ThisKind<T>(PhantomData<T>);
impl<T> ParameterKind for ThisKind<T> {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }
}

pub struct ThisMutKind<T>(PhantomData<T>);
impl<T> ParameterKind for ThisMutKind<T> {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }
}

pub struct OptionalKind<T>(PhantomData<T>);
impl<T> ParameterKind for OptionalKind<T> {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::optional()
    }
}

pub struct RestKind<T>(PhantomData<T>);
impl<T> ParameterKind for RestKind<T> {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }
}

/// Marker for [`JsContext`] as a function parameter (does not count toward arity rules).
pub struct JsContextParamKind;

impl ParameterKind for JsContextParamKind {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }
}

pub trait GetParam<'js, E: JsEngine>: Sized {
    type Kind: ParameterKind;
    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self>;
}

impl<'js, T, E> GetParam<'js, E> for T
where
    E: JsEngine,
    T: FromJsValue<'js, E> + Sized,
    T: JsParameterType,
{
    type Kind = Regular<T>;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        let value = accessor.next_arg().unwrap(); // it's safe, since RustFunc::call ensures
        let ctx = accessor.context();
        let v = JsValue::from_raw(ctx.clone(), value);
        T::from_js_value(ctx, v)
    }
}

impl<'js, E: JsEngine> GetParam<'js, E> for JsContext<'js, E> {
    type Kind = JsContextParamKind;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        Ok(accessor.context())
    }
}

impl<'js, T, E> GetParam<'js, E> for This<T>
where
    E: JsEngine,
    T: FromJsValue<'js, E> + JsParameterType,
{
    type Kind = ThisKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        let value = accessor.get_this();
        let ctx = accessor.context();
        let val = T::from_js_value(
            ctx.clone(),
            JsValue::from_raw(ctx.clone(), value),
        )?;
        Ok(Self(val))
    }
}

impl<'js, T, E> GetParam<'js, E> for ThisMut<'js, T, E>
where
    E::Value: JsObjectOps,
    E: JsEngine,
    T: JsClass<E>,
{
    type Kind = ThisMutKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        let value = accessor.get_this();
        let ctx = accessor.context();
        let obj = JsObject::from_js_value(
            ctx.clone(),
            JsValue::from_raw(ctx.clone(), value),
        )?;
        if !crate::Class::instance_of::<T>(&obj) {
            return Err(crate::HostError::new(
                crate::error::E_TYPE,
                format!("Not instance of {}", std::any::type_name::<T>()),
            )
            .with_name("TypeError")
            .into());
        }
        Ok(ThisMut(obj, PhantomData))
    }
}

impl<'js, T, E> GetParam<'js, E> for Optional<T>
where
    E: JsEngine,
    T: FromJsValue<'js, E>,
{
    type Kind = OptionalKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        let ctx = accessor.context();
        match accessor.next_arg() {
            Some(v) => T::from_js_value(ctx.clone(), JsValue::from_raw(ctx.clone(), v))
                .map(|t| Optional(Some(t))),
            None => Ok(Optional(None)),
        }
    }
}

impl<'js, T, E> GetParam<'js, E> for Rest<T>
where
    E: JsEngine,
    T: FromJsValue<'js, E>,
{
    type Kind = RestKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        let mut values = Vec::new();
        let ctx = accessor.context();
        if accessor.is_last_param {
            while let Some(value) = accessor.next_arg() {
                let v = JsValue::from_raw(ctx.clone(), value);
                values.push(T::from_js_value(ctx.clone(), v)?);
            }
        }
        Ok(Rest(values))
    }
}

// Allow Vec<T> as a direct parameter, interpreting a single JS Array argument.
// This avoids requiring `impl<T> JsParameterType for Vec<T>`.
impl<'js, T, E> GetParam<'js, E> for Vec<T>
where
    E: JsEngine,
    E::Value: JsTypeOf + JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    type Kind = Regular<Vec<T>>;

    fn get_param(accessor: &mut ParamsAccessor<'js, E>) -> JsResult<Self> {
        let value = accessor.next_arg().unwrap(); // safe: call site ensures arg exists
        let ctx = accessor.context();
        <Vec<T> as FromJsValue<'js, E>>::from_js_value(
            ctx.clone(),
            JsValue::from_raw(ctx.clone(), value),
        )
    }
}

/// Marker trait for types that can be used as JsFunc function parameters.
/// When used with JsFunc::new, the parameter types will be automatically
/// converted from JsValue to their Rust equivalents.
pub trait JsParameterType {}

impl JsParameterType for () {}
impl JsParameterType for i8 {}
impl JsParameterType for u8 {}
impl JsParameterType for i16 {}
impl JsParameterType for u16 {}
impl JsParameterType for i32 {}
impl JsParameterType for u32 {}
impl JsParameterType for i64 {}
impl JsParameterType for u64 {}
impl JsParameterType for f32 {}
impl JsParameterType for f64 {}
impl JsParameterType for bool {}
impl JsParameterType for String {}
impl JsParameterType for isize {}
impl JsParameterType for usize {}

/// for IntoJSArg
/// &str does not implement FromJsValue
impl JsParameterType for &str {}

/// `Option<T>` can be used as a parameter type for async functions
impl<T> JsParameterType for Option<T> where T: JsParameterType {}

impl<'js, T, E> FromJsValue<'js, E> for JsClassRef<'js, T, E>
where
    E::Value: JsObjectOps,
    E: JsEngine,
    T: JsClass<E>,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        let obj = JsObject::from_js_value(ctx, value)?;
        if !crate::Class::instance_of::<T>(&obj) {
            return Err(crate::HostError::new(
                crate::error::E_TYPE,
                format!("Not instance of {}", std::any::type_name::<T>()),
            )
            .with_name("TypeError")
            .into());
        }
        Ok(Self(obj, PhantomData))
    }
}

impl<'js, T, E> JsParameterType for JsClassRef<'js, T, E> where E: JsEngine {}

macro_rules! impl_from_params {
    ($($T:ident),*) => {
        impl<Eng: JsEngine, $($T,)*> FromParams<Eng> for ($($T,)*)
        where
            $(for<'js> $T: GetParam<'js, Eng>,)*
        {
            #[allow(unused_variables)]
            fn from_params<'js>(accessor: &mut ParamsAccessor<'js, Eng>) -> JsResult<Self> {
                let param_count = count_idents!($($T),*);
                #[allow(unused_mut)]
                let mut current_param = 0;

                Ok(($(
                    {
                        current_param += 1;
                        accessor.set_last_param(current_param == param_count);
                        $T::get_param(accessor)?
                    },
                )*))
            }

            fn param_requirements() -> ParamRequirement {

                #[allow(unused_mut)]
                let mut req = ParamRequirement {
                    required_count: 0,
                    exhaustive: true,
                };

                $(
                    let param_req = <$T::Kind>::param_requirement();
                    req.required_count += param_req.required_count;
                    if !param_req.exhaustive {
                        req.exhaustive = false;
                    }
                )*
                req
            }
        }
    };
}

// Helper macro to count identifiers
macro_rules! count_idents {
    () => { 0 };
    ($head:ident $(,$tail:ident)*) => { 1 + count_idents!($($tail),*) };
}

// Implement for common tuple sizes
impl_from_params!();
impl_from_params!(A);
impl_from_params!(A, B);
impl_from_params!(A, B, C);
impl_from_params!(A, B, C, D);
impl_from_params!(A, B, C, D, E);
impl_from_params!(A, B, C, D, E, F);
impl_from_params!(A, B, C, D, E, F, G);
impl_from_params!(A, B, C, D, E, F, G, H);
