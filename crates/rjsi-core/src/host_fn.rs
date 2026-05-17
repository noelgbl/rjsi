use std::marker::PhantomData;

use crate::args::RawHostFn;
use crate::convert::Coerced;
use crate::{Args, Context, Engine, Error, FromJs, Result, ToJs, Value};

pub struct CallSite<'cx, 'a, E: Engine> {
    pub cx: &'a mut Context<'cx, E>,
    this: Option<Value<'cx, E>>,
    args: &'a Args<'cx, E>,
    pub cursor: usize,
}

impl<'cx, 'a, E: Engine> CallSite<'cx, 'a, E> {
    #[inline]
    pub fn new(cx: &'a mut Context<'cx, E>, this: Value<'cx, E>, args: &'a Args<'cx, E>) -> Self {
        Self {
            cx,
            this: Some(this),
            args,
            cursor: 0,
        }
    }

    #[inline]
    pub fn take_this(&mut self) -> Result<Value<'cx, E>> {
        self.this
            .take()
            .ok_or_else(|| Error::type_err("`this` already extracted"))
    }
}

pub trait FromCallSite<'cx, E: Engine>: Sized {
    fn from_call_site(site: &mut CallSite<'cx, '_, E>) -> Result<Self>;
}

#[inline]
pub fn extract_positional<'cx, E: Engine, T: FromJs<'cx, E>>(
    site: &mut CallSite<'cx, '_, E>,
) -> Result<T> {
    let idx = site.cursor;
    let val = site.args.get(idx).ok_or_else(|| missing_arg(idx))?;
    site.cursor += 1;
    T::from_js(site.cx, val)
}

macro_rules! impl_from_call_site_primitive {
    ($($T:ty),* $(,)?) => {
        $(
            impl<'cx, E: Engine> FromCallSite<'cx, E> for $T {
                #[inline]
                fn from_call_site(site: &mut CallSite<'cx, '_, E>) -> Result<Self> {
                    extract_positional(site)
                }
            }
        )*
    };
}

impl_from_call_site_primitive!(
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

impl<'cx, E: Engine> FromCallSite<'cx, E> for Value<'cx, E> {
    #[inline]
    fn from_call_site(site: &mut CallSite<'cx, '_, E>) -> Result<Self> {
        extract_positional(site)
    }
}

pub struct This<T>(pub T);

impl<T> std::ops::Deref for This<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::DerefMut for This<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

pub struct Opt<T>(pub Option<T>);

pub struct Rest<T>(pub Vec<T>);

impl<'cx, E: Engine, T: FromJs<'cx, E>> FromCallSite<'cx, E> for This<T> {
    #[inline]
    fn from_call_site(site: &mut CallSite<'cx, '_, E>) -> Result<Self> {
        let this = site.take_this()?;
        T::from_js(site.cx, this).map(This)
    }
}

impl<'cx, E: Engine, T: FromJs<'cx, E>> FromCallSite<'cx, E> for Opt<T> {
    #[inline]
    fn from_call_site(site: &mut CallSite<'cx, '_, E>) -> Result<Self> {
        match site.args.get(site.cursor) {
            None => Ok(Opt(None)),
            Some(v) => {
                site.cursor += 1;
                T::from_js(site.cx, v).map(|v| Opt(Some(v)))
            }
        }
    }
}

impl<'cx, E: Engine, T: FromJs<'cx, E>> FromCallSite<'cx, E> for Rest<T> {
    #[inline]
    fn from_call_site(site: &mut CallSite<'cx, '_, E>) -> Result<Self> {
        let mut out = Vec::new();
        while let Some(v) = site.args.get(site.cursor) {
            site.cursor += 1;
            out.push(T::from_js(site.cx, v)?);
        }
        Ok(Rest(out))
    }
}

pub struct WithCx<Args>(PhantomData<fn() -> Args>);

pub trait HostFn<E: Engine, Marker>: Sized + 'static {
    fn call_typed<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: &Args<'rt, E>,
    ) -> Result<Value<'rt, E>>;
}

pub struct HostFnAdapter<F, Marker>(pub F, pub PhantomData<fn() -> Marker>);

impl<E: Engine, F, Marker> RawHostFn<E> for HostFnAdapter<F, Marker>
where
    F: HostFn<E, Marker>,
{
    #[inline]
    fn call<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: Args<'rt, E>,
    ) -> Result<Value<'rt, E>> {
        self.0.call_typed(cx, this, &args)
    }
}

impl<E, F, R> HostFn<E, ()> for F
where
    E: Engine,
    F: FnMut() -> R + 'static,
    R: for<'cx> ToJs<'cx, E>,
{
    #[inline]
    fn call_typed<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        _this: Value<'rt, E>,
        _args: &Args<'rt, E>,
    ) -> Result<Value<'rt, E>> {
        (self)().to_js(cx)
    }
}

impl<E, F, R> HostFn<E, WithCx<()>> for F
where
    E: Engine,
    F: for<'cx> FnMut(&mut Context<'cx, E>) -> R + 'static,
    R: for<'cx> ToJs<'cx, E>,
{
    #[inline]
    fn call_typed<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        _this: Value<'rt, E>,
        _args: &Args<'rt, E>,
    ) -> Result<Value<'rt, E>> {
        (self)(cx).to_js(cx)
    }
}

#[inline]
fn missing_arg(idx: usize) -> crate::Error {
    crate::Error::type_err(match idx {
        0 => "missing argument 0",
        1 => "missing argument 1",
        2 => "missing argument 2",
        3 => "missing argument 3",
        4 => "missing argument 4",
        5 => "missing argument 5",
        6 => "missing argument 6",
        7 => "missing argument 7",
        _ => "missing argument",
    })
}

macro_rules! impl_host_fn {
    ($marker:ty ; $($A:ident),+) => {
        // Without context: FnMut(A1, A2, ...) -> R
        #[allow(non_snake_case)]
        impl<E, F, $($A,)+ R> HostFn<E, $marker> for F
        where
            E: Engine,
            F: FnMut($($A),+) -> R + 'static,
            $( $A: for<'cx> FromCallSite<'cx, E>, )+
            R: for<'cx> ToJs<'cx, E>,
        {
            #[inline]
            fn call_typed<'rt>(
                &mut self,
                cx: &mut Context<'rt, E>,
                this: Value<'rt, E>,
                args: &Args<'rt, E>,
            ) -> Result<Value<'rt, E>> {
                let mut site = CallSite::new(cx, this, args);
                $( let $A = $A::from_call_site(&mut site)?; )+
                (self)($($A),+).to_js(site.cx)
            }
        }

        // With context: FnMut(&mut Context<E>, A1, A2, ...) -> R
        #[allow(non_snake_case)]
        impl<E, F, $($A,)+ R> HostFn<E, WithCx<$marker>> for F
        where
            E: Engine,
            F: for<'cx> FnMut(&mut Context<'cx, E>, $($A),+) -> R + 'static,
            $( $A: for<'cx> FromCallSite<'cx, E>, )+
            R: for<'cx> ToJs<'cx, E>,
        {
            #[inline]
            fn call_typed<'rt>(
                &mut self,
                cx: &mut Context<'rt, E>,
                this: Value<'rt, E>,
                args: &Args<'rt, E>,
            ) -> Result<Value<'rt, E>> {
                let mut site = CallSite::new(cx, this, args);
                $( let $A = $A::from_call_site(&mut site)?; )+
                (self)(site.cx, $($A),+).to_js(site.cx)
            }
        }
    };
}

impl_host_fn!((A1,); A1);
impl_host_fn!((A1, A2); A1, A2);
impl_host_fn!((A1, A2, A3); A1, A2, A3);
impl_host_fn!((A1, A2, A3, A4); A1, A2, A3, A4);
impl_host_fn!((A1, A2, A3, A4, A5); A1, A2, A3, A4, A5);
impl_host_fn!((A1, A2, A3, A4, A5, A6); A1, A2, A3, A4, A5, A6);
impl_host_fn!((A1, A2, A3, A4, A5, A6, A7); A1, A2, A3, A4, A5, A6, A7);
impl_host_fn!((A1, A2, A3, A4, A5, A6, A7, A8); A1, A2, A3, A4, A5, A6, A7, A8);
