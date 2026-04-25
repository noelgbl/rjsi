use crate::{
    FromJsValue, HostError, IntoJsValue, JsContext, JsEngine, JsObject, JsObjectOps, JsResult,
    JsTypeOf, JsValue, JsValueImpl, TypedArrayElement,
};

use std::ops::{Deref, DerefMut};

pub struct JsArrayBuffer<'js, E: JsEngine + 'static> {
    inner: JsObject<'js, E>,
}

impl<'js, E: JsEngine> Deref for JsArrayBuffer<'js, E> {
    type Target = JsObject<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'js, E: JsEngine> DerefMut for JsArrayBuffer<'js, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'js, E: JsEngine> Clone for JsArrayBuffer<'js, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsArrayBuffer<'js, E>
where
    E::Value: JsValueImpl,
{
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.inner.into_js_value(_ctx)
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsArrayBuffer<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_array_buffer() {
            Ok(Self {
                inner: JsObject::from_js_value(ctx, value)?,
            })
        } else {
            Err(HostError::not_array_buffer().into())
        }
    }
}

/// Trait for JavaScript array buffer operations.
pub trait JsArrayBufferOps: JsValueImpl {
    /// Create an ArrayBuffer by copying existing data.
    fn from_bytes(ctx: &Self::Context, bytes: &[u8]) -> Self;

    /// Create an ArrayBuffer from an existing Vec without copying when possible.
    fn from_vec(ctx: &Self::Context, vec: Vec<u8>) -> Self;

    /// Get the byte length of the ArrayBuffer.
    fn length(&self) -> usize;

    /// Get a safe slice view of the ArrayBuffer's data.
    fn as_slice(&self) -> &[u8];

    /// Get a mutable slice view of the ArrayBuffer's data.
    fn as_mut_slice(&mut self) -> &mut [u8];
}

impl<'js, E: JsEngine> JsArrayBuffer<'js, E>
where
    E::Value: JsObjectOps + JsArrayBufferOps + JsTypeOf,
{
    /// Create a new ArrayBuffer by copying the provided bytes.
    pub fn from_bytes(ctx: JsContext<'js, E>, bytes: &[u8]) -> JsResult<Self> {
        let value = E::Value::from_bytes(ctx.native_context(), bytes);
        if value.is_exception() {
            return Err(crate::RjsiJSError::from_thrown_value(
                ctx.clone(),
                JsValue::from_raw(ctx.clone(), value),
            ));
        }
        let v = JsValue::from_raw(ctx.clone(), value);
        Self::from_js_value(ctx, v)
    }

    /// Create a new ArrayBuffer from owned bytes.
    pub fn from_bytes_owned<B: Into<Vec<u8>>>(ctx: JsContext<'js, E>, data: B) -> JsResult<Self> {
        let value = E::Value::from_vec(ctx.native_context(), data.into());
        if value.is_exception() {
            return Err(crate::RjsiJSError::from_thrown_value(
                ctx.clone(),
                JsValue::from_raw(ctx.clone(), value),
            ));
        }
        let v = JsValue::from_raw(ctx.clone(), value);
        Self::from_js_value(ctx, v)
    }

    /// Get the byte length of the ArrayBuffer.
    pub fn len(&self) -> usize {
        self.as_value().length()
    }

    /// Check if the ArrayBuffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a safe slice view of the ArrayBuffer's data.
    pub fn as_slice(&self) -> &[u8] {
        self.as_value().as_slice()
    }

    /// Get a reference to the ArrayBuffer's raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }

    /// Get a mutable slice view of the ArrayBuffer's data.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut_value().as_mut_slice()
    }

    /// Get a slice of the ArrayBuffer from start to end.
    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        &self.as_slice()[start..end]
    }

    /// Copy the contents of the ArrayBuffer into a new Vec.
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    /// Compute how many `T` elements this buffer can represent when aligned.
    pub fn element_count<T>(&self) -> JsResult<usize>
    where
        T: TypedArrayElement,
    {
        if !self.len().is_multiple_of(T::BYTES_PER_ELEMENT) {
            return Err(HostError::typed_array_alignment_error().into());
        }

        Ok(self.len() / T::BYTES_PER_ELEMENT)
    }

    /// Validate if the given byte offset is properly aligned for `T`.
    pub fn validate_alignment<T>(&self, offset: usize) -> bool
    where
        T: TypedArrayElement,
    {
        offset.is_multiple_of(T::BYTES_PER_ELEMENT)
    }

    /// Construct a JsArrayBuffer from a JsObject if it is an ArrayBuffer.
    pub fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        if obj.as_value().is_array_buffer() {
            Some(Self { inner: obj })
        } else {
            None
        }
    }
}

impl<'js, E: JsEngine> crate::function::JsParameterType for JsArrayBuffer<'js, E> {}
