use smallvec::SmallVec;

use crate::{
    error::HostError,
    runtime::Runtime,
    scope::ScopeLike,
    convert::{FromJs, IntoJs},
};

pub struct Args<'s, R: Runtime> {
    raw: SmallVec<[R::Value<'s>; 8]>,
    this: R::Value<'s>,
}

impl<'s, R: Runtime> Args<'s, R> {
    pub fn new<I>(this: R::Value<'s>, values: I) -> Self
    where
        I: IntoIterator<Item = R::Value<'s>>,
    {
        Self {
            raw: values.into_iter().collect(),
            this,
        }
    }

    pub fn from_smallvec(this: R::Value<'s>, raw: SmallVec<[R::Value<'s>; 8]>) -> Self {
        Self { raw, this }
    }

    #[inline]
    pub fn get<T: FromJs<'s, R>>(
        &self,
        scope: &mut R::Scope<'s, '_>,
        index: usize,
    ) -> Result<T, R::Error> {
        let value = self
            .raw
            .get(index)
            .cloned()
            .unwrap_or_else(|| scope.undefined());
        T::from_js(scope, value)
    }

    #[inline]
    pub fn value(&self, index: usize) -> Option<R::Value<'s>> {
        self.raw.get(index).cloned()
    }

    #[inline]
    pub fn this(&self) -> R::Value<'s> {
        self.this.clone()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    #[inline]
    pub fn as_slice(&self) -> &[R::Value<'s>] {
        &self.raw
    }
}

pub type Callback<R> = dyn for<'s> Fn(
        &mut <R as Runtime>::Scope<'s, 's>,
        Args<'s, R>,
    ) -> Result<<R as Runtime>::Value<'s>, <R as Runtime>::Error>
    + Send
    + Sync
    + 'static;

pub fn bind<R, A, Ret, F>(f: F) -> impl for<'s> Fn(
    &mut R::Scope<'s, 's>,
    Args<'s, R>,
) -> Result<R::Value<'s>, R::Error>
+ Send
+ Sync
+ 'static
where
    R: Runtime,
    A: for<'s> FromJsTuple<'s, R>,
    Ret: for<'s> IntoJs<'s, R>,
    F: for<'s> Fn(&mut R::Scope<'s, 's>, A) -> Result<Ret, R::Error> + Send + Sync + 'static,
{
    move |scope, args| {
        let typed = A::from_js_args(scope, &args)?;
        let ret = f(scope, typed)?;
        ret.into_js(scope)
    }
}

pub trait FromJsTuple<'s, R: Runtime>: Sized {
    fn from_js_args(scope: &mut R::Scope<'s, '_>, args: &Args<'s, R>) -> Result<Self, R::Error>;
}

impl<'s, R: Runtime> FromJsTuple<'s, R> for () {
    fn from_js_args(
        _scope: &mut R::Scope<'s, '_>,
        _args: &Args<'s, R>,
    ) -> Result<Self, R::Error> {
        Ok(())
    }
}

macro_rules! impl_from_js_tuple {
    ($count:expr => $(($idx:tt, $name:ident)),+) => {
        impl<'s, R, $( $name, )+> FromJsTuple<'s, R> for ($( $name, )+)
        where
            R: Runtime,
            $( $name: FromJs<'s, R>, )+
        {
            fn from_js_args(
                scope: &mut R::Scope<'s, '_>,
                args: &Args<'s, R>,
            ) -> Result<Self, R::Error> {
                if args.len() < $count {
                    return Err(HostError::invalid_arg_count($count, args.len()).into());
                }
                Ok(($(
                    args.get::<$name>(scope, $idx)?,
                )+))
            }
        }
    };
}

impl_from_js_tuple!(1 => (0, A0));
impl_from_js_tuple!(2 => (0, A0), (1, A1));
impl_from_js_tuple!(3 => (0, A0), (1, A1), (2, A2));
impl_from_js_tuple!(4 => (0, A0), (1, A1), (2, A2), (3, A3));
impl_from_js_tuple!(5 => (0, A0), (1, A1), (2, A2), (3, A3), (4, A4));
impl_from_js_tuple!(6 => (0, A0), (1, A1), (2, A2), (3, A3), (4, A4), (5, A5));
impl_from_js_tuple!(7 => (0, A0), (1, A1), (2, A2), (3, A3), (4, A4), (5, A5), (6, A6));
impl_from_js_tuple!(8 => (0, A0), (1, A1), (2, A2), (3, A3), (4, A4), (5, A5), (6, A6), (7, A7));
