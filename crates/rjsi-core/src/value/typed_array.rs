use crate::{
    FromJsValue, HostError, IntoJsValue, JsArrayBuffer, JsArrayBufferOps, JsContext, JsEngine,
    JsObject, JsObjectOps, JsResult, JsTypeOf, JsValue, JsValueImpl, JsValueMapper,
};
use std::marker::PhantomData;
use std::ops::Deref;

/// Represents the different kinds of TypedArrays available in JavaScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsTypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    BigInt64,
    BigUint64,
    Float32,
    Float64,
}

/// Trait for types that can be used as compile-time typed array view markers.
pub trait TypedArrayElement: Sized {
    const BYTES_PER_ELEMENT: usize;
    const TYPE: JsTypedArrayKind;
}

pub struct Uint8Clamped;

impl TypedArrayElement for i8 {
    const BYTES_PER_ELEMENT: usize = 1;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Int8;
}

impl TypedArrayElement for u8 {
    const BYTES_PER_ELEMENT: usize = 1;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Uint8;
}

impl TypedArrayElement for Uint8Clamped {
    const BYTES_PER_ELEMENT: usize = 1;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Uint8Clamped;
}

impl TypedArrayElement for i16 {
    const BYTES_PER_ELEMENT: usize = 2;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Int16;
}

impl TypedArrayElement for u16 {
    const BYTES_PER_ELEMENT: usize = 2;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Uint16;
}

impl TypedArrayElement for i32 {
    const BYTES_PER_ELEMENT: usize = 4;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Int32;
}

impl TypedArrayElement for u32 {
    const BYTES_PER_ELEMENT: usize = 4;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Uint32;
}

impl TypedArrayElement for f32 {
    const BYTES_PER_ELEMENT: usize = 4;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Float32;
}

impl TypedArrayElement for f64 {
    const BYTES_PER_ELEMENT: usize = 8;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::Float64;
}

impl TypedArrayElement for i64 {
    const BYTES_PER_ELEMENT: usize = 8;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::BigInt64;
}

impl TypedArrayElement for u64 {
    const BYTES_PER_ELEMENT: usize = 8;
    const TYPE: JsTypedArrayKind = JsTypedArrayKind::BigUint64;
}

impl JsTypedArrayKind {
    pub fn bytes_per_element(&self) -> usize {
        match self {
            JsTypedArrayKind::Int8 | JsTypedArrayKind::Uint8 | JsTypedArrayKind::Uint8Clamped => 1,
            JsTypedArrayKind::Int16 | JsTypedArrayKind::Uint16 => 2,
            JsTypedArrayKind::Int32 | JsTypedArrayKind::Uint32 | JsTypedArrayKind::Float32 => 4,
            JsTypedArrayKind::BigInt64
            | JsTypedArrayKind::BigUint64
            | JsTypedArrayKind::Float64 => 8,
        }
    }
}

pub struct AnyJsTypedArray<'js, E: JsEngine + 'static>(JsObject<'js, E>);

impl<'js, E: JsEngine> Clone for AnyJsTypedArray<'js, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js, E: JsEngine> Deref for AnyJsTypedArray<'js, E> {
    type Target = JsObject<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for AnyJsTypedArray<'js, E>
where
    E::Value: JsTypeOf + JsTypedArrayOps,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_object() && value.as_value().get_kind().is_some() {
            JsObject::from_js_value(ctx, value).map(Self)
        } else {
            Err(HostError::not_typed_array().into())
        }
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for AnyJsTypedArray<'js, E>
where
    E::Value: JsValueImpl,
{
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0.into_js_value(_ctx)
    }
}

pub struct JsTypedArray<'js, E: JsEngine + 'static, T: TypedArrayElement = u8> {
    inner: AnyJsTypedArray<'js, E>,
    marker: PhantomData<T>,
}

impl<'js, E: JsEngine, T: TypedArrayElement> Clone for JsTypedArray<'js, E, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<'js, E: JsEngine, T: TypedArrayElement> Deref for JsTypedArray<'js, E, T> {
    type Target = AnyJsTypedArray<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'js, E: JsEngine, T> FromJsValue<'js, E> for JsTypedArray<'js, E, T>
where
    E::Value: JsTypeOf + JsTypedArrayOps,
    T: TypedArrayElement,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        let inner = AnyJsTypedArray::from_js_value(ctx, value)?;
        Self::from_any(inner)
    }
}

impl<'js, E: JsEngine, T> IntoJsValue<'js, E> for JsTypedArray<'js, E, T>
where
    E::Value: JsValueImpl,
    T: TypedArrayElement,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.inner.into_js_value(ctx)
    }
}

impl<'js, E: JsEngine, T> JsTypedArray<'js, E, T>
where
    E::Value: JsTypedArrayOps,
    T: TypedArrayElement,
{
    pub fn from_any(inner: AnyJsTypedArray<'js, E>) -> JsResult<Self> {
        let actual = inner.kind();
        if actual != T::TYPE {
            return Err(HostError::typed_array_kind_mismatch(T::TYPE, actual).into());
        }

        Ok(Self {
            inner,
            marker: PhantomData,
        })
    }
}

/// Trait for JavaScript typed array operations.
pub trait JsTypedArrayOps: JsValueImpl {
    fn from_array_buffer(
        ctx: &Self::Context,
        kind: JsTypedArrayKind,
        buffer: Self,
        byte_offset: usize,
        length: Option<usize>,
    ) -> Self;

    fn get_kind(&self) -> Option<JsTypedArrayKind>;

    fn get_array_buffer(&self) -> Option<Self>;

    fn get_byte_offset(&self) -> usize;

    fn get_length(&self) -> usize;

    fn get_byte_length(&self) -> usize;
}

impl<'js, E: JsEngine> AnyJsTypedArray<'js, E>
where
    E::Value: JsTypedArrayOps,
{
    pub fn kind(&self) -> JsTypedArrayKind {
        self.as_value().get_kind().expect("Invalid typed array")
    }

    pub fn byte_offset(&self) -> usize {
        self.as_value().get_byte_offset()
    }

    pub fn len(&self) -> usize {
        self.as_value().get_length()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn byte_length(&self) -> usize {
        self.as_value().get_byte_length()
    }

    pub fn bytes_per_element(&self) -> usize {
        self.kind().bytes_per_element()
    }

    pub fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        if obj.as_value().get_kind().is_some() {
            Some(Self(obj))
        } else {
            None
        }
    }
}

impl<'js, E: JsEngine> AnyJsTypedArray<'js, E>
where
    E::Value: JsObjectOps + JsTypedArrayOps + JsArrayBufferOps,
{
    pub fn from_array_buffer(
        ctx: JsContext<'js, E>,
        kind: JsTypedArrayKind,
        buffer: JsArrayBuffer<'js, E>,
        byte_offset: usize,
        length: Option<usize>,
    ) -> JsResult<Self> {
        let length = resolve_typed_array_length(kind, &buffer, byte_offset, length)?;

        let buffer_value = buffer.into_js_value(ctx.clone()).into_inner();
        let value = E::Value::from_array_buffer(
            ctx.native_context(),
            kind,
            buffer_value,
            byte_offset,
            Some(length),
        );
        let ctx_b = ctx.clone();
        let jv = value.try_map_js(ctx.clone(), move |v| JsValue::from_raw(ctx_b, v))?;
        Self::from_js_value(ctx, jv)
    }

    pub fn buffer(&self) -> JsResult<JsArrayBuffer<'js, E>> {
        let buffer = self
            .as_value()
            .get_array_buffer()
            .ok_or_else(|| -> crate::RjsiJSError { HostError::not_array_buffer().into() })?;
        let ctx = self.context();
        let v = JsValue::from_raw(ctx.clone(), buffer);
        JsArrayBuffer::from_js_value(ctx, v)
    }

    pub fn byte_view(&self) -> Option<&[u8]> {
        let buffer = self.as_value().get_array_buffer()?;
        let offset = self.byte_offset();
        let length = self.byte_length();
        let view = buffer.as_slice().get(offset..offset + length)?;
        Some(unsafe { std::slice::from_raw_parts(view.as_ptr(), view.len()) })
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.byte_view()
    }

    pub fn cast<T>(&self) -> JsResult<JsTypedArray<'js, E, T>>
    where
        T: TypedArrayElement,
    {
        JsTypedArray::from_any(self.clone())
    }
}

impl<'js, E: JsEngine, T> JsTypedArray<'js, E, T>
where
    E::Value: JsObjectOps + JsTypedArrayOps + JsArrayBufferOps,
    T: TypedArrayElement,
{
    pub fn from_array_buffer(
        ctx: JsContext<'js, E>,
        buffer: JsArrayBuffer<'js, E>,
        byte_offset: usize,
        length: Option<usize>,
    ) -> JsResult<Self> {
        AnyJsTypedArray::from_array_buffer(ctx, T::TYPE, buffer, byte_offset, length).map(|inner| {
            Self {
                inner,
                marker: PhantomData,
            }
        })
    }

    pub fn kind(&self) -> JsTypedArrayKind {
        T::TYPE
    }

    pub fn bytes_per_element(&self) -> usize {
        T::BYTES_PER_ELEMENT
    }

    pub fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        let inner = AnyJsTypedArray::from_object(obj)?;
        Self::from_any(inner).ok()
    }

    pub fn into_any(self) -> AnyJsTypedArray<'js, E> {
        self.inner
    }
}

fn resolve_typed_array_length<'js, E>(
    kind: JsTypedArrayKind,
    buffer: &JsArrayBuffer<'js, E>,
    byte_offset: usize,
    length: Option<usize>,
) -> JsResult<usize>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayBufferOps + JsValueImpl,
{
    let bytes_per_element = kind.bytes_per_element();
    if !byte_offset.is_multiple_of(bytes_per_element) {
        return Err(HostError::typed_array_alignment_error().into());
    }

    let buffer_size = buffer.len();
    if byte_offset > buffer_size {
        return Err(HostError::typed_array_range_error().into());
    }

    let available_bytes = buffer_size - byte_offset;
    match length {
        Some(length) => {
            if length > available_bytes / bytes_per_element {
                return Err(HostError::typed_array_range_error().into());
            }
            Ok(length)
        }
        None => {
            if !available_bytes.is_multiple_of(bytes_per_element) {
                return Err(HostError::typed_array_alignment_error().into());
            }
            Ok(available_bytes / bytes_per_element)
        }
    }
}

impl<'js, E: JsEngine> crate::function::JsParameterType for AnyJsTypedArray<'js, E> {}
impl<'js, E: JsEngine, T: TypedArrayElement> crate::function::JsParameterType
    for JsTypedArray<'js, E, T>
{
}
