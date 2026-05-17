use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct Func<F, P>(pub(crate) F, pub(crate) PhantomData<fn() -> P>);

impl<F, P> Func<F, P> {
    pub fn new(f: F) -> Self {
        Func(f, PhantomData)
    }
}

impl<F, P> From<F> for Func<F, P> {
    fn from(f: F) -> Self {
        Func(f, PhantomData)
    }
}

pub struct WithCx<P>(PhantomData<fn() -> P>);

pub struct ThisState<S, P = ()>(PhantomData<fn() -> (S, P)>);

pub struct ThisStateMut<S, P = ()>(PhantomData<fn() -> (S, P)>);

pub struct This<T>(pub T);

pub struct Opt<T>(pub Option<T>);

pub struct Rest<T>(pub Vec<T>);

pub struct Flat<T>(pub T);

pub struct Exhaustive;

pub struct MutFn<F>(pub RefCell<F>);

impl<F> MutFn<F> {
    pub fn new(f: F) -> Self {
        Self(RefCell::new(f))
    }
}

impl<F> From<F> for MutFn<F> {
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

pub struct OnceFn<F>(pub Cell<Option<F>>);

impl<F> OnceFn<F> {
    pub fn new(f: F) -> Self {
        Self(Cell::new(Some(f)))
    }
}

impl<F> From<F> for OnceFn<F> {
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

macro_rules! wrapper_traits {
    ($name:ident<$T:ident>($inner:ty)) => {
        impl<$T> $name<$T> {
            pub fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl<$T> AsRef<$inner> for $name<$T> {
            fn as_ref(&self) -> &$inner {
                &self.0
            }
        }

        impl<$T> AsMut<$inner> for $name<$T> {
            fn as_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }

        impl<$T> Deref for $name<$T> {
            type Target = $inner;
            fn deref(&self) -> &$inner {
                &self.0
            }
        }

        impl<$T> DerefMut for $name<$T> {
            fn deref_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }

        impl<$T> From<$inner> for $name<$T> {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

wrapper_traits!(This<T>(T));
wrapper_traits!(Opt<T>(Option<T>));
wrapper_traits!(Rest<T>(Vec<T>));
wrapper_traits!(Flat<T>(T));
