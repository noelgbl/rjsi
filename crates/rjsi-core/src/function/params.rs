// Contains code from requickjs.
// https://github.com/DelSkayn/rquickjs/blob/master/core/src/value/function/params.rs

use super::types::{Exhaustive, Flat, Opt, Rest, This};
use crate::convert::Coerced;
use crate::{Args, Context, Engine, Error, FromJs, Result, Value};

pub struct Params<'a, 'cx, E: Engine> {
    cx: &'a mut Context<'cx, E>,
    this: Option<Value<'cx, E>>,
    args: &'a Args<'cx, E>,
}

impl<'a, 'cx, E: Engine> Params<'a, 'cx, E> {
    pub fn new(cx: &'a mut Context<'cx, E>, this: Value<'cx, E>, args: &'a Args<'cx, E>) -> Self {
        Self {
            cx,
            this: Some(this),
            args,
        }
    }

    pub fn check_params(&self, req: ParamRequirement) -> Result<()> {
        let given = self.args.len();
        if given < req.min {
            return Err(Error::missing_args(req.min, given));
        }
        if req.exhaustive && given > req.max {
            return Err(Error::too_many_args(req.max, given));
        }
        Ok(())
    }

    pub fn ctx(&mut self) -> &mut Context<'cx, E> {
        self.cx
    }

    pub fn this(&self) -> Option<&Value<'cx, E>> {
        self.this.as_ref()
    }

    pub fn arg(&self, index: usize) -> Option<Value<'cx, E>> {
        self.args.get(index)
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    pub fn access(self) -> ParamsAccessor<'a, 'cx, E> {
        ParamsAccessor {
            params: self,
            offset: 0,
        }
    }
}

pub struct ParamsAccessor<'a, 'cx, E: Engine> {
    params: Params<'a, 'cx, E>,
    offset: usize,
}

impl<'a, 'cx, E: Engine> ParamsAccessor<'a, 'cx, E> {
    pub fn ctx(&mut self) -> &mut Context<'cx, E> {
        self.params.cx
    }

    pub fn take_this(&mut self) -> Result<Value<'cx, E>> {
        self.params
            .this
            .take()
            .ok_or_else(|| Error::type_err("`this` already extracted"))
    }

    pub fn arg(&mut self) -> Value<'cx, E> {
        let v = self
            .params
            .args
            .get(self.offset)
            .expect("arg called too many times");
        self.offset += 1;
        v
    }

    pub fn len(&self) -> usize {
        self.params.args.len().saturating_sub(self.offset)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParamRequirement {
    pub(crate) min: usize,
    pub(crate) max: usize,
    pub(crate) exhaustive: bool,
}

impl ParamRequirement {
    pub const fn single() -> Self {
        Self {
            min: 1,
            max: 1,
            exhaustive: false,
        }
    }

    pub const fn exhaustive() -> Self {
        Self {
            min: 0,
            max: 0,
            exhaustive: true,
        }
    }

    pub const fn optional() -> Self {
        Self {
            min: 0,
            max: 1,
            exhaustive: false,
        }
    }

    pub const fn any() -> Self {
        Self {
            min: 0,
            max: usize::MAX,
            exhaustive: false,
        }
    }

    pub const fn none() -> Self {
        Self {
            min: 0,
            max: 0,
            exhaustive: false,
        }
    }

    pub const fn combine(self, other: Self) -> Self {
        Self {
            min: self.min.saturating_add(other.min),
            max: self.max.saturating_add(other.max),
            exhaustive: self.exhaustive || other.exhaustive,
        }
    }

    pub fn min(&self) -> usize {
        self.min
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn is_exhaustive(&self) -> bool {
        self.exhaustive
    }
}

pub trait FromParam<'cx, E: Engine>: Sized {
    fn param_requirement() -> ParamRequirement;

    fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self>;
}

pub trait FromParams<'cx, E: Engine>: Sized {
    fn param_requirements() -> ParamRequirement;

    fn from_params<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self>;
}

macro_rules! impl_from_param_primitive {
    ($($T:ty),* $(,)?) => {
        $(
            impl<'cx, E: Engine> FromParam<'cx, E> for $T {
                fn param_requirement() -> ParamRequirement {
                    ParamRequirement::single()
                }

                fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
                    let v = params.arg();
                    <$T as FromJs<'cx, E>>::from_js(params.ctx(), v)
                }
            }
        )*
    };
}

impl_from_param_primitive!(
    bool,
    i8,
    u8,
    i16,
    u16,
    i32,
    u32,
    i64,
    u64,
    isize,
    usize,
    f64,
    String,
    (),
    Coerced<bool>,
    Coerced<i32>,
    Coerced<f64>,
    Coerced<String>,
);

impl<'cx, E: Engine> FromParam<'cx, E> for Value<'cx, E> {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::single()
    }

    fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
        Ok(params.arg())
    }
}

impl<'cx, E: Engine, T> FromParam<'cx, E> for This<T>
where
    T: FromJs<'cx, E>,
{
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }

    fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
        let this = params.take_this()?;
        T::from_js(params.ctx(), this).map(This)
    }
}

impl<'cx, E: Engine, T> FromParam<'cx, E> for Opt<T>
where
    T: FromJs<'cx, E>,
{
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::optional()
    }

    fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
        if params.is_empty() {
            Ok(Opt(None))
        } else {
            let v = params.arg();
            T::from_js(params.ctx(), v).map(|v| Opt(Some(v)))
        }
    }
}

impl<'cx, E: Engine, T> FromParam<'cx, E> for Rest<T>
where
    T: FromJs<'cx, E>,
{
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }

    fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
        let mut out = Vec::with_capacity(params.len());
        while !params.is_empty() {
            let v = params.arg();
            out.push(T::from_js(params.ctx(), v)?);
        }
        Ok(Rest(out))
    }
}

impl<'cx, E: Engine> FromParam<'cx, E> for Exhaustive {
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::exhaustive()
    }

    fn from_param<'a>(_params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
        Ok(Exhaustive)
    }
}

impl<'cx, E: Engine, T> FromParam<'cx, E> for Flat<T>
where
    T: FromParams<'cx, E>,
{
    fn param_requirement() -> ParamRequirement {
        T::param_requirements()
    }

    fn from_param<'a>(params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
        T::from_params(params).map(Flat)
    }
}

macro_rules! impl_from_params_tuple {
    ($($t:ident),*) => {
        #[allow(non_snake_case)]
        impl<'cx, E: Engine $(, $t)*> FromParams<'cx, E> for ($($t,)*)
        where
            $($t: FromParam<'cx, E>,)*
        {
            fn param_requirements() -> ParamRequirement {
                ParamRequirement::none()
                    $(.combine(<$t as FromParam<'cx, E>>::param_requirement()))*
            }

            fn from_params<'a>(_params: &mut ParamsAccessor<'a, 'cx, E>) -> Result<Self> {
                Ok((
                    $(<$t as FromParam<'cx, E>>::from_param(_params)?,)*
                ))
            }
        }
    };
}

impl_from_params_tuple!();
impl_from_params_tuple!(A);
impl_from_params_tuple!(A, B);
impl_from_params_tuple!(A, B, C);
impl_from_params_tuple!(A, B, C, D);
impl_from_params_tuple!(A, B, C, D, E0);
impl_from_params_tuple!(A, B, C, D, E0, F);
impl_from_params_tuple!(A, B, C, D, E0, F, G);
impl_from_params_tuple!(A, B, C, D, E0, F, G, H);
