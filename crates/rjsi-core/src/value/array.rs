use crate::{
    FromJsValue, HostError, IntoJsValue, JsContext, JsEngine, JsObject, JsObjectOps, JsResult,
    JsTypeOf, JsValue, JsValueImpl, JsValueMapper,
};
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

pub struct JsArray<'js, E: JsEngine + 'static>(JsObject<'js, E>);

impl<'js, E: JsEngine> Deref for JsArray<'js, E> {
    type Target = JsObject<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> Clone for JsArray<'js, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsArray<'js, E> {
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0.into_js_value(_ctx)
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsArray<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_array() {
            JsObject::from_js_value(ctx, value).map(Self)
        } else {
            Err(HostError::not_array().into())
        }
    }
}

/// Trait for primitive JavaScript array index operations.
pub trait JsArrayOps: JsValueImpl {
    /// Create a new empty array.
    fn new_array(ctx: &Self::Context) -> Self;

    /// Get element at index.
    ///
    /// Returns the element value or an exception.
    fn get_index(&self, index: u32) -> Self;

    /// Set element at index.
    ///
    /// Returns `undefined` on success or an exception.
    fn set_index(&self, index: u32, value: Self) -> Self;
}

impl<'js, E: JsEngine> JsArray<'js, E>
where
    E::Value: JsObjectOps + JsArrayOps + JsTypeOf,
{
    /// Create a new empty JavaScript array.
    pub fn new(ctx: JsContext<'js, E>) -> JsResult<Self> {
        let value = E::Value::new_array(ctx.native_context());
        if value.is_exception() {
            return Err(crate::RjsiJSError::from_thrown_value(
                ctx.clone(),
                JsValue::from_raw(ctx.clone(), value),
            ));
        }
        let v = JsValue::from_raw(ctx.clone(), value);
        Self::from_js_value(ctx, v)
    }

    /// Get the JavaScript `length` property.
    pub fn len(&self) -> JsResult<u32> {
        self.0.get::<_, u32>("length")
    }

    /// Check whether the array is empty.
    pub fn is_empty(&self) -> JsResult<bool> {
        self.len().map(|len| len == 0)
    }

    /// Get the raw JS value at the given index.
    pub fn get_value(&self, index: u32) -> JsResult<JsValue<'js, E>> {
        let ctx = self.context();
        self.as_value()
            .get_index(index)
            .try_map_js(ctx.clone(), |value| JsValue::from_raw(ctx.clone(), value))
    }

    /// Set the raw JS value at the given index.
    pub fn set_value(&self, index: u32, value: JsValue<'js, E>) -> JsResult<()> {
        self.as_value()
            .set_index(index, value.into_inner())
            .try_map_js(self.context(), |_| ())
    }

    /// Get an optionally-present element with Rust conversion semantics.
    pub fn get_opt<T>(&self, index: u32) -> JsResult<Option<T>>
    where
        T: FromJsValue<'js, E>,
    {
        if !self.has_index(index)? {
            return Ok(None);
        }

        let ctx = self.context();
        let value = self.get_value(index)?;
        T::from_js_value(ctx, value).map(Some)
    }

    /// Set an element after Rust-to-JS conversion.
    pub fn set<T>(&self, index: u32, value: T) -> JsResult<()>
    where
        T: IntoJsValue<'js, E>,
    {
        let ctx = self.context();
        self.set_value(index, value.into_js_value(ctx))
    }

    /// Delete an array index using primitive object semantics.
    pub fn delete(&self, index: u32) -> JsResult<bool> {
        self.0.delete(index)
    }

    /// Check whether an index is present using primitive object semantics.
    pub fn has_index(&self, index: u32) -> JsResult<bool> {
        self.0.has_property(index)
    }

    /// Push a raw JS value using primitive index writes.
    pub fn push_value(&self, value: JsValue<'js, E>) -> JsResult<u32> {
        let index = self.len()?;
        self.set_value(index, value)?;
        Ok(index + 1)
    }

    /// Push a Rust value and return the new array length.
    pub fn push<T>(&self, value: T) -> JsResult<u32>
    where
        T: IntoJsValue<'js, E>,
    {
        let ctx = self.context();
        self.push_value(value.into_js_value(ctx))
    }

    /// Pop a raw JS value using primitive index operations.
    pub fn pop_value(&self) -> JsResult<JsValue<'js, E>> {
        let len = self.len()?;
        let ctx = self.context();
        if len == 0 {
            return Ok(JsValue::undefined(ctx));
        }

        let index = len - 1;
        let value = self.get_value(index)?;
        self.delete(index)?;
        self.0.set("length", index)?;
        Ok(value)
    }

    /// Pop an optionally-present element with Rust conversion semantics.
    pub fn pop_opt<T>(&self) -> JsResult<Option<T>>
    where
        T: FromJsValue<'js, E>,
    {
        if self.is_empty()? {
            return Ok(None);
        }

        let ctx = self.context();
        let value = self.pop_value()?;
        T::from_js_value(ctx, value).map(Some)
    }

    /// Iterate over typed values in `[0, length)`.
    pub fn iter<T>(&self) -> JsResult<ArrayIter<'js, E, T>>
    where
        T: FromJsValue<'js, E>,
    {
        Ok(ArrayIter {
            array: self.clone(),
            index: 0,
            count: self.len()?,
            marker: PhantomData,
        })
    }

    /// Iterate over raw values in `[0, length)`.
    pub fn iter_values(&self) -> JsResult<ArrayValueIter<'js, E>> {
        Ok(ArrayValueIter {
            array: self.clone(),
            index: 0,
            count: self.len()?,
        })
    }

    /// Iterate over present values only, skipping holes.
    pub fn iter_present<T>(&self) -> JsResult<ArrayPresentIter<'js, E, T>>
    where
        T: FromJsValue<'js, E>,
    {
        Ok(ArrayPresentIter {
            array: self.clone(),
            index: 0,
            count: self.len()?,
            marker: PhantomData,
        })
    }

    /// Construct a JsArray from a JsObject if it is an array.
    pub fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        if obj.as_value().is_array() {
            Some(Self(obj))
        } else {
            None
        }
    }
}

/// Iterator over typed JavaScript values in an array.
pub struct ArrayIter<'js, E: JsEngine, T>
where
    E::Value: JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    array: JsArray<'js, E>,
    index: u32,
    count: u32,
    marker: PhantomData<T>,
}

impl<'js, E, T> Iterator for ArrayIter<'js, E, T>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    type Item = JsResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let ctx = self.array.context();
            let result = self
                .array
                .get_value(self.index)
                .and_then(|value| T::from_js_value(ctx, value));
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'js, E, T> ExactSizeIterator for ArrayIter<'js, E, T>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    fn len(&self) -> usize {
        (self.count - self.index) as usize
    }
}

/// Iterator over raw JavaScript values in an array.
pub struct ArrayValueIter<'js, E: JsEngine>
where
    E::Value: JsObjectOps + JsArrayOps,
{
    array: JsArray<'js, E>,
    index: u32,
    count: u32,
}

impl<'js, E> Iterator for ArrayValueIter<'js, E>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
{
    type Item = JsResult<JsValue<'js, E>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let result = self.array.get_value(self.index);
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'js, E> ExactSizeIterator for ArrayValueIter<'js, E>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
{
    fn len(&self) -> usize {
        (self.count - self.index) as usize
    }
}

/// Iterator over present JavaScript array entries converted into Rust values.
pub struct ArrayPresentIter<'js, E: JsEngine, T>
where
    E::Value: JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    array: JsArray<'js, E>,
    index: u32,
    count: u32,
    marker: PhantomData<T>,
}

impl<'js, E, T> Iterator for ArrayPresentIter<'js, E, T>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    type Item = JsResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let index = self.index;
            self.index += 1;

            let has_index = match self.array.has_index(index) {
                Ok(has_index) => has_index,
                Err(err) => return Some(Err(err)),
            };

            if has_index {
                let ctx = self.array.context();
                let value = match self.array.get_value(index) {
                    Ok(value) => value,
                    Err(err) => return Some(Err(err)),
                };
                return Some(T::from_js_value(ctx, value));
            }
        }

        None
    }
}

/// Converts a Rust Vec into a JavaScript array.
impl<'js, E, T> IntoJsValue<'js, E> for Vec<T>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
    T: IntoJsValue<'js, E>,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let array = JsArray::new(ctx.clone()).expect("array");
        for item in self {
            array.push(item).expect("Failed to push value into array");
        }
        <JsArray<'js, E> as IntoJsValue<'js, E>>::into_js_value(array, ctx)
    }
}

/// Converts a JavaScript array to a Rust Vec using dense array semantics.
impl<'js, E, T> FromJsValue<'js, E> for Vec<T>
where
    E: JsEngine,
    E::Value: JsTypeOf + JsObjectOps + JsArrayOps,
    T: FromJsValue<'js, E>,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_array() {
            let array = JsArray::from_js_value(ctx, value)?;
            array.iter::<T>()?.collect::<JsResult<Vec<_>>>()
        } else {
            Err(HostError::not_array().into())
        }
    }
}

impl<'js, E: JsEngine> crate::function::JsParameterType for JsArray<'js, E> {}

impl<'js, E: JsEngine> fmt::Display for JsArray<'js, E>
where
    E::Value: JsTypeOf + crate::JsValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.deref().fmt(f)
    }
}

impl<'js, E: JsEngine> fmt::Debug for JsArray<'js, E>
where
    E::Value: JsTypeOf + crate::JsValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JsArray({})", self)
    }
}
