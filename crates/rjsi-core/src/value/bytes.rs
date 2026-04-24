//! Opaque bytes wrapper for Rust/JS interop.
//!
//! Typical flow:
//! 1. Rust creates `JsBytes` from an existing `Bytes` payload.
//! 2. JavaScript receives the object and forwards it without unpacking.
//! 3. Rust accepts `JsBytes` or `Bytes` again and continues processing.
//!
//! The main scenario is `rust -> js -> rust`.
//! JavaScript is not expected to interpret the payload structure here; it only
//! carries an opaque object across API boundaries.
//!
//! Boundaries:
//! - `JsBytes` is a transport type, not a semantic type.
//! - It does not mean JSON, protobuf, UTF-8 text, or any other schema.
//! - If callers need structured meaning, that should be expressed by the API
//!   using `JsBytes`, not by `JsBytes` itself.
//!
//! Typical uses:
//! - Rust produces a request body, JS routes it, Rust sends it onward.
//! - Rust returns bytes to JS, JS passes them into another Rust callback.
//! - A text payload is converted to bytes once and then carried opaquely.
//!
use bytes::Bytes;

use crate::function::Constructor;
use crate::{
    Class, ClassSetup, FromJsValue, HostError, IntoJsValue, JsArrayOps, JsClass, JsContext,
    JsEngine, JsErrorFactory, JsExceptionThrower, JsFunc, JsObject, JsObjectOps, JsResult,
    JsTypeOf, JsValue, JsValueConversion, JsValueImpl, PropertyDescriptor, RjsiJSError,
};

use std::ops::Deref;

#[derive(Clone)]
pub(crate) struct JsBytesData {
    bytes: Bytes,
}

#[derive(Clone)]
/// JavaScript-visible opaque byte handle.
pub struct JsBytes<'js, E: JsEngine + 'static> {
    inner: JsObject<'js, E>,
}

impl<'js, E: JsEngine> Deref for JsBytes<'js, E> {
    type Target = JsObject<'js, E>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'js, E: JsEngine> JsBytes<'js, E>
where
    E::Value: JsValueImpl + JsObjectOps + JsTypeOf + JsValueConversion + JsArrayOps + 'static,
    E::Context: JsErrorFactory + JsExceptionThrower,
{
    /// Byte length of the wrapped payload.
    pub fn len(&self) -> JsResult<usize> {
        Ok(self.inner.borrow::<JsBytesData>()?.bytes.len())
    }

    pub fn is_empty(&self) -> JsResult<bool> {
        Ok(self.inner.borrow::<JsBytesData>()?.bytes.is_empty())
    }

    /// Clone out the underlying bytes.
    pub fn to_bytes(&self) -> JsResult<Bytes> {
        Ok(self.inner.borrow::<JsBytesData>()?.bytes.clone())
    }

    pub(crate) fn to_vec(&self) -> JsResult<Vec<u8>> {
        Ok(self.to_bytes()?.to_vec())
    }

    /// Decode the payload as UTF-8 text.
    pub fn to_string(&self) -> JsResult<String> {
        String::from_utf8(self.to_vec()?).map_err(|err| {
            HostError::new(
                crate::error::E_TYPE,
                format!("JsBytes contains invalid UTF-8: {}", err),
            )
            .with_name("TypeError")
            .into()
        })
    }

    pub(crate) fn from_object(obj: JsObject<'js, E>) -> Option<Self> {
        if Class::instance_of::<JsBytesData>(&obj) {
            Some(Self { inner: obj })
        } else {
            None
        }
    }
}

impl<'js, E: JsEngine> JsBytes<'js, E>
where
    E::Value: JsValueImpl + JsObjectOps + JsTypeOf + JsValueConversion + JsArrayOps + 'static,
    E::Context: JsErrorFactory + JsExceptionThrower,
{
    /// Create `JsBytes` from an existing Rust `Bytes` payload.
    pub fn from_bytes(ctx: JsContext<'js, E>, bytes: Bytes) -> JsResult<Self> {
        ctx.register_hidden_class::<JsBytesData>()?;
        let instance = Class::lookup::<JsBytesData>(&ctx)?.instance(JsBytesData { bytes });
        Ok(Self { inner: instance })
    }

    /// Create `JsBytes` from UTF-8 text by storing the underlying text bytes.
    pub fn from_string<S>(ctx: JsContext<'js, E>, text: S) -> JsResult<Self>
    where
        S: Into<String>,
    {
        Self::from_bytes(ctx, Bytes::from(text.into()))
    }
}

impl<E: JsEngine> JsClass<E> for JsBytesData
where
    E::Value: JsValueImpl + JsObjectOps + JsTypeOf + JsValueConversion + JsArrayOps + 'static,
    E::Context: JsErrorFactory + JsExceptionThrower,
{
    const NAME: &'static str = "JsBytes";

    fn data_constructor() -> Constructor<E> {
        Constructor::callback(0, |_, _, _| {
            crate::illegal_constructor("JsBytes cannot be constructed from JavaScript")
        })
    }

    fn call_without_new() -> Constructor<E> {
        Constructor::callback(0, |_, _, _| {
            crate::illegal_constructor("JsBytes cannot be constructed from JavaScript")
        })
    }

    fn class_setup(class: &ClassSetup<'_, '_, E>) -> JsResult<()> {
        // Use JsFunc::callback instead of new_func/method: IntoJsCallable requires
        // `for<'js> P: GetParam<'js, E>`, which JsBytes<'setup, E> cannot satisfy when tied
        // to the class-setup lifetime inside a 'static closure.
        let getter = JsFunc::callback(class.context().clone(), 0, |ctx, this, _args| {
            let this = this.ok_or_else(|| -> RjsiJSError {
                HostError::new(
                    crate::error::E_TYPE,
                    "JsBytes length getter requires a receiver",
                )
                .with_name("TypeError")
                .into()
            })?;
            let bytes = JsBytes::from_object(this).ok_or_else(|| -> RjsiJSError {
                HostError::new(crate::error::E_TYPE, "Not a JsBytes instance")
                    .with_name("TypeError")
                    .into()
            })?;
            let n = bytes.len()?;
            Ok(JsValue::from_rust(ctx, n))
        })?;
        class.property(
            "length",
            PropertyDescriptor::from_getter(getter).configurable(),
        )?;

        class.callback_method("toString", 0, |ctx, this, _args| {
            let this = this.ok_or_else(|| -> RjsiJSError {
                HostError::new(
                    crate::error::E_TYPE,
                    "JsBytes.toString requires a receiver",
                )
                .with_name("TypeError")
                .into()
            })?;
            let bytes = JsBytes::from_object(this).ok_or_else(|| -> RjsiJSError {
                HostError::new(crate::error::E_TYPE, "Not a JsBytes instance")
                    .with_name("TypeError")
                    .into()
            })?;
            let s = bytes.to_string()?;
            Ok(JsValue::from_rust(ctx, s))
        })?;

        Ok(())
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsBytes<'js, E>
where
    E::Value: JsValueImpl,
{
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.inner.into_js_value(_ctx)
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsBytes<'js, E>
where
    E::Value: JsValueImpl + JsTypeOf + JsObjectOps + JsValueConversion + JsArrayOps + 'static,
    E::Context: JsErrorFactory + JsExceptionThrower,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        let obj = JsObject::from_js_value(ctx, value)?;
        Self::from_object(obj).ok_or_else(|| {
            HostError::new(crate::error::E_TYPE, "Value is not a JsBytes instance")
                .with_name("TypeError")
                .into()
        })
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for Bytes
where
    E::Value: JsValueImpl + JsTypeOf + JsObjectOps + JsValueConversion + JsArrayOps + 'static,
    E::Context: JsErrorFactory + JsExceptionThrower,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        JsBytes::from_js_value(ctx, value)?.to_bytes()
    }
}

impl<'js, E: JsEngine> crate::function::JsParameterType for JsBytes<'js, E> {}
impl crate::function::JsParameterType for Bytes {}
