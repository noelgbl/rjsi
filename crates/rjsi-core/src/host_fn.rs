use std::marker::PhantomData;

use crate::args::RawHostFn;
use crate::{Args, Context, Engine, FromJs, Result, ToJs, Value};

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

pub trait HostFn<E: Engine, Marker>: Sized + 'static {
    fn call_typed<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: &Args<'rt, E>,
    ) -> Result<Value<'rt, E>>;
}

pub trait HostFnWithCx<E: Engine, Marker>: Sized + 'static {
    fn call_typed<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: &Args<'rt, E>,
    ) -> Result<Value<'rt, E>>;
}

pub struct HostFnWithCxAdapter<F, M>(pub F, pub PhantomData<fn() -> M>);

impl<E: Engine, F, M> RawHostFn<E> for HostFnWithCxAdapter<F, M>
where
    F: HostFnWithCx<E, M>,
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

pub struct WithCx<Args>(PhantomData<fn() -> Args>);

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

impl<E, F, R> HostFnWithCx<E, WithCx<()>> for F
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

macro_rules! impl_host_fn {
    (no_cx: $marker:ty ; $($A:ident),+ ; $($idx:expr),+) => {
        #[allow(non_snake_case)]
        impl<E, F, $($A,)+ R> HostFn<E, $marker> for F
        where
            E: Engine,
            F: FnMut($($A),+) -> R + 'static,
            $( $A: for<'cx> FromJs<'cx, E>, )+
            R: for<'cx> ToJs<'cx, E>,
        {
            #[inline]
            fn call_typed<'rt>(
                &mut self,
                cx: &mut Context<'rt, E>,
                _this: Value<'rt, E>,
                args: &Args<'rt, E>,
            ) -> Result<Value<'rt, E>> {
                $(
                    let $A = $A::from_js(cx, args.get($idx).ok_or_else(|| missing_arg($idx))?)?;
                )+
                (self)($($A),+).to_js(cx)
            }
        }
    };

    (cx: $inner:ty ; $($A:ident),+ ; $($idx:expr),+) => {
        #[allow(non_snake_case)]
        impl<E, F, $($A,)+ R> HostFnWithCx<E, WithCx<$inner>> for F
        where
            E: Engine,
            F: for<'cx> FnMut(&mut Context<'cx, E>, $($A),+) -> R + 'static,
            $( $A: for<'cx> FromJs<'cx, E>, )+
            R: for<'cx> ToJs<'cx, E>,
        {
            #[inline]
            fn call_typed<'rt>(
                &mut self,
                cx: &mut Context<'rt, E>,
                _this: Value<'rt, E>,
                args: &Args<'rt, E>,
            ) -> Result<Value<'rt, E>> {
                $(
                    let $A = $A::from_js(cx, args.get($idx).ok_or_else(|| missing_arg($idx))?)?;
                )+
                (self)(cx, $($A),+).to_js(cx)
            }
        }
    };
}

impl_host_fn!(no_cx: (A1,); A1; 0);
impl_host_fn!(no_cx: (A1, A2); A1, A2; 0, 1);
impl_host_fn!(no_cx: (A1, A2, A3); A1, A2, A3; 0, 1, 2);
impl_host_fn!(no_cx: (A1, A2, A3, A4); A1, A2, A3, A4; 0, 1, 2, 3);
impl_host_fn!(no_cx: (A1, A2, A3, A4, A5); A1, A2, A3, A4, A5; 0, 1, 2, 3, 4);
impl_host_fn!(no_cx: (A1, A2, A3, A4, A5, A6); A1, A2, A3, A4, A5, A6; 0, 1, 2, 3, 4, 5);
impl_host_fn!(no_cx: (A1, A2, A3, A4, A5, A6, A7); A1, A2, A3, A4, A5, A6, A7; 0, 1, 2, 3, 4, 5, 6);
impl_host_fn!(no_cx: (A1, A2, A3, A4, A5, A6, A7, A8); A1, A2, A3, A4, A5, A6, A7, A8; 0, 1, 2, 3, 4, 5, 6, 7);

impl_host_fn!(cx: (A1,); A1; 0);
impl_host_fn!(cx: (A1, A2); A1, A2; 0, 1);
impl_host_fn!(cx: (A1, A2, A3); A1, A2, A3; 0, 1, 2);
impl_host_fn!(cx: (A1, A2, A3, A4); A1, A2, A3, A4; 0, 1, 2, 3);
impl_host_fn!(cx: (A1, A2, A3, A4, A5); A1, A2, A3, A4, A5; 0, 1, 2, 3, 4);
impl_host_fn!(cx: (A1, A2, A3, A4, A5, A6); A1, A2, A3, A4, A5, A6; 0, 1, 2, 3, 4, 5);
impl_host_fn!(cx: (A1, A2, A3, A4, A5, A6, A7); A1, A2, A3, A4, A5, A6, A7; 0, 1, 2, 3, 4, 5, 6);
impl_host_fn!(cx: (A1, A2, A3, A4, A5, A6, A7, A8); A1, A2, A3, A4, A5, A6, A7, A8; 0, 1, 2, 3, 4, 5, 6, 7);
