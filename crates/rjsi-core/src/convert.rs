use smallvec::SmallVec;

pub use crate::{FromJs, IntoJs};
use crate::{HostError, JsEngine, JsResult, JsScope, ParamsAccessor, RjsiJSError};

pub trait ScopeExt<'js>: JsScope<'js> {
    fn get(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &str,
    ) -> Result<
        Option<<Self::Engine as JsEngine>::Value<'js>>,
        <Self::Engine as JsEngine>::Value<'js>,
    > {
        let key = self.property_key(key);
        self.get_property(object, &key)
    }

    fn set<V: IntoJs<'js, Self>>(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &str,
        value: V,
    ) -> Result<(), <Self::Engine as JsEngine>::Value<'js>>
    where
        Self: Sized,
    {
        let key = self.property_key(key);
        let value = value.into_js(self).map_err(|err| {
            let message = self.string(&err.to_string());
            self.throw(message)
        })?;
        self.set_property(object, &key, &value)
    }

    fn has(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &str,
    ) -> Result<bool, <Self::Engine as JsEngine>::Value<'js>> {
        let key = self.property_key(key);
        self.has_property(object, &key)
    }

    fn delete(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &str,
    ) -> Result<bool, <Self::Engine as JsEngine>::Value<'js>> {
        let key = self.property_key(key);
        self.delete_property(object, &key)
    }

    fn call<A: JsArgs<'js, Self>>(
        &mut self,
        function: &<Self::Engine as JsEngine>::Value<'js>,
        this: Option<&<Self::Engine as JsEngine>::Value<'js>>,
        args: A,
    ) -> Result<<Self::Engine as JsEngine>::Value<'js>, <Self::Engine as JsEngine>::Value<'js>>
    where
        Self: Sized,
    {
        let args = args.into_args(self).map_err(|err| {
            let message = self.string(&err.to_string());
            self.throw(message)
        })?;
        self.call_function(function, this, &args)
    }

    fn call_method<A: JsArgs<'js, Self>>(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &str,
        args: A,
    ) -> Result<<Self::Engine as JsEngine>::Value<'js>, <Self::Engine as JsEngine>::Value<'js>>
    where
        Self: Sized,
    {
        let function = match self.get(object, key)? {
            Some(function) => function,
            None => {
                let message = self.string("method not found");
                return Err(self.throw(message));
            }
        };
        self.call(&function, Some(object), args)
    }
}

impl<'js, S: JsScope<'js>> ScopeExt<'js> for S {}

pub trait JsArgs<'js, S: JsScope<'js>> {
    fn into_args(
        self,
        scope: &mut S,
    ) -> JsResult<SmallVec<[<S::Engine as JsEngine>::Value<'js>; 4]>>;
}

impl<'js, S: JsScope<'js>> JsArgs<'js, S> for () {
    fn into_args(
        self,
        _scope: &mut S,
    ) -> JsResult<SmallVec<[<S::Engine as JsEngine>::Value<'js>; 4]>> {
        Ok(SmallVec::new())
    }
}

impl<'js, S, A> JsArgs<'js, S> for (A,)
where
    S: JsScope<'js>,
    A: IntoJs<'js, S>,
{
    fn into_args(
        self,
        scope: &mut S,
    ) -> JsResult<SmallVec<[<S::Engine as JsEngine>::Value<'js>; 4]>> {
        let mut args = SmallVec::new();
        args.push(self.0.into_js(scope)?);
        Ok(args)
    }
}

impl<'js, S, A, B> JsArgs<'js, S> for (A, B)
where
    S: JsScope<'js>,
    A: IntoJs<'js, S>,
    B: IntoJs<'js, S>,
{
    fn into_args(
        self,
        scope: &mut S,
    ) -> JsResult<SmallVec<[<S::Engine as JsEngine>::Value<'js>; 4]>> {
        let mut args = SmallVec::new();
        args.push(self.0.into_js(scope)?);
        args.push(self.1.into_js(scope)?);
        Ok(args)
    }
}

pub trait ParamsExt<'a, 'js, E: JsEngine>
where
    'js: 'a,
{
    fn next<T: FromJs<'js, E::Scope<'js>>>(&mut self) -> JsResult<T>;
}

impl<'a, 'js, E: JsEngine> ParamsExt<'a, 'js, E> for ParamsAccessor<'a, 'js, E>
where
    'js: 'a,
{
    fn next<T: FromJs<'js, E::Scope<'js>>>(&mut self) -> JsResult<T> {
        let value = self
            .next_arg()
            .ok_or_else(|| RjsiJSError::from(conversion_error("argument")))?;
        T::from_js(self.scope(), &value)
    }
}

fn conversion_error(expected: &str) -> HostError {
    HostError::type_error(crate::error::E_TYPE, format!("expected {expected}"))
}

pub mod prelude {
    pub use super::{FromJs, IntoJs, JsArgs, ParamsExt, ScopeExt};
}
