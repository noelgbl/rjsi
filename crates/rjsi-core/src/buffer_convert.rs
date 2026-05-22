use crate::capabilities::{
    ArrayBuffer, BigInt64Array, BigUint64Array, Buffers, Float32Array, Float64Array, Int8Array, Int16Array, Int32Array, TypedArrayKind, Uint8Array, Uint8ClampedArray, Uint16Array, Uint32Array
};
use crate::context::ContextBufferExt;
use crate::{Context, Engine, Error, FromJs, Object, Result, ToJs, Value};

impl<'cx, E: Buffers> ToJs<'cx, E> for Vec<u8> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        Ok(cx.uint8_array_from_vec(self)?.into_value())
    }
}

impl<'cx, E: Buffers> ToJs<'cx, E> for Box<[u8]> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        let buf = cx.array_buffer_from_boxed(self)?;
        let length = buf.byte_length(cx)?;
        let ta = E::typed_array_new(
            cx,
            TypedArrayKind::Uint8,
            buf.into_object().into_raw(),
            0,
            length,
        )?;
        Ok(Object::new(ta).into_value())
    }
}

impl<'cx, E: Buffers> ToJs<'cx, E> for bytes::Bytes {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        let buf = cx.array_buffer_from_bytes(self)?;
        let length = buf.byte_length(cx)?;
        let ta = E::typed_array_new(
            cx,
            TypedArrayKind::Uint8,
            buf.into_object().into_raw(),
            0,
            length,
        )?;
        Ok(Object::new(ta).into_value())
    }
}

impl<'cx, E: Buffers> FromJs<'cx, E> for Vec<u8> {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        if E::value_is_array_buffer(value.as_raw()) {
            let obj = value
                .as_object()
                .ok_or_else(|| Error::type_err("expected ArrayBuffer object"))?;
            let buf = ArrayBuffer::<E>::new(obj);
            return buf.to_vec(cx);
        }
        if let Some(_kind) = E::value_typed_array_kind(value.as_raw()) {
            let obj = value
                .as_object()
                .ok_or_else(|| Error::type_err("expected TypedArray object"))?;
            let info = E::typed_array_info(cx, obj.as_raw())?;
            let mut v = vec![0u8; info.byte_length];
            E::typed_array_copy_to(cx, obj.as_raw(), &mut v)?;
            return Ok(v);
        }
        Err(Error::type_err("expected ArrayBuffer or TypedArray"))
    }
}

impl<'cx, E: Buffers> FromJs<'cx, E> for ArrayBuffer<'cx, E> {
    fn from_js(_cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        if !E::value_is_array_buffer(value.as_raw()) {
            return Err(Error::type_err("expected ArrayBuffer"));
        }
        let obj = value
            .as_object()
            .ok_or_else(|| Error::type_err("expected ArrayBuffer object"))?;
        Ok(ArrayBuffer::new(obj))
    }
}

macro_rules! impl_typed_array_from_js {
    ($name:ident, $kind:ident) => {
        impl<'cx, E: Buffers> FromJs<'cx, E> for $name<'cx, E> {
            fn from_js(_cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
                match E::value_typed_array_kind(value.as_raw()) {
                    Some(TypedArrayKind::$kind) => {
                        let obj = value.as_object().ok_or_else(|| {
                            Error::type_err(concat!("expected ", stringify!($name), " object",))
                        })?;
                        Ok($name::new(obj))
                    }
                    _ => Err(Error::type_err(concat!("expected ", stringify!($name),))),
                }
            }
        }
    };
}

impl_typed_array_from_js!(Int8Array, Int8);
impl_typed_array_from_js!(Uint8Array, Uint8);
impl_typed_array_from_js!(Uint8ClampedArray, Uint8Clamped);
impl_typed_array_from_js!(Int16Array, Int16);
impl_typed_array_from_js!(Uint16Array, Uint16);
impl_typed_array_from_js!(Int32Array, Int32);
impl_typed_array_from_js!(Uint32Array, Uint32);
impl_typed_array_from_js!(Float32Array, Float32);
impl_typed_array_from_js!(Float64Array, Float64);
impl_typed_array_from_js!(BigInt64Array, BigInt64);
impl_typed_array_from_js!(BigUint64Array, BigUint64);

#[allow(dead_code)]
const _: fn() = || {
    fn _assert<E: Engine>() {}
};
