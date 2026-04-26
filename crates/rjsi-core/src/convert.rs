use crate::{
    error::{E_TYPE, HostError},
    runtime::Runtime,
    scope::ScopeLike,
    value::ValueLike,
};

pub trait IntoJs<'s, R: Runtime>: Sized {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error>;
}

pub trait FromJs<'s, R: Runtime>: Sized {
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error>;
}

impl<'s, R: Runtime> IntoJs<'s, R> for () {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        Ok(scope.undefined())
    }
}

impl<'s, R: Runtime> IntoJs<'s, R> for bool {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        Ok(scope.boolean(self))
    }
}

impl<'s, R: Runtime> IntoJs<'s, R> for i32 {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        Ok(scope.integer(self))
    }
}

impl<'s, R: Runtime> IntoJs<'s, R> for f64 {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        Ok(scope.number(self))
    }
}

impl<'s, R: Runtime> IntoJs<'s, R> for &str {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        Ok(scope.string(self))
    }
}

impl<'s, R: Runtime> IntoJs<'s, R> for String {
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        Ok(scope.string(&self))
    }
}

impl<'s, R, T> IntoJs<'s, R> for Option<T>
where
    R: Runtime,
    T: IntoJs<'s, R>,
{
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        match self {
            Some(value) => value.into_js(scope),
            None => Ok(scope.null()),
        }
    }
}

impl<'s, R, T> IntoJs<'s, R> for Vec<T>
where
    R: Runtime,
    T: IntoJs<'s, R>,
{
    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
        let array = scope.array(self.len() as u32);
        for (index, value) in self.into_iter().enumerate() {
            let value = value.into_js(scope)?;
            array.set_index(scope, index as u32, value);
        }
        Ok(array)
    }
}

impl<'s, R: Runtime> FromJs<'s, R> for bool {
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
        if value.is_boolean() {
            value
                .as_bool(scope)
                .ok_or_else(|| HostError::type_error(E_TYPE, "expected boolean").into())
        } else {
            Err(HostError::type_error(E_TYPE, "expected boolean").into())
        }
    }
}

impl<'s, R: Runtime> FromJs<'s, R> for i32 {
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
        value
            .as_i32(scope)
            .ok_or_else(|| HostError::type_error(E_TYPE, "expected integer").into())
    }
}

impl<'s, R: Runtime> FromJs<'s, R> for f64 {
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
        value
            .as_f64(scope)
            .ok_or_else(|| HostError::type_error(E_TYPE, "expected number").into())
    }
}

impl<'s, R: Runtime> FromJs<'s, R> for String {
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
        value
            .with_str(scope, str::to_owned)
            .ok_or_else(|| HostError::type_error(E_TYPE, "expected string").into())
    }
}

impl<'s, R, T> FromJs<'s, R> for Option<T>
where
    R: Runtime,
    T: FromJs<'s, R>,
{
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
        if value.is_null() || value.is_undefined() {
            Ok(None)
        } else {
            T::from_js(scope, value).map(Some)
        }
    }
}

impl<'s, R, T> FromJs<'s, R> for Vec<T>
where
    R: Runtime,
    T: FromJs<'s, R>,
{
    fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
        if !value.is_array() {
            return Err(HostError::type_error(E_TYPE, "expected array").into());
        }
        let len = value.length(scope);
        let mut out = Vec::with_capacity(len as usize);
        for index in 0..len {
            let item = value.get_index(scope, index);
            out.push(T::from_js(scope, item)?);
        }
        Ok(out)
    }
}

pub struct ZeroCopyBuf<'a>(pub &'a [u8]);
