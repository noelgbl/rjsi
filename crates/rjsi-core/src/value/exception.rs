use super::{JSError, JsErrorFactory};
use crate::{
    FromJsValue, HostError, IntoJsValue, JsContext, JsEngine, JsObject, JsObjectOps, JsResult,
    JsTypeOf, JsValue, JsValueImpl,
};
use std::fmt;
use std::ops::Deref;
use std::string::String;

/// Represents a JavaScript exception object wrapper
pub struct JsException<'js, E: JsEngine>(JsObject<'js, E>);

impl<'js, E: JsEngine> Deref for JsException<'js, E> {
    type Target = JsObject<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsException<'js, E>
where
    E::Value: JsValueImpl,
{
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0.into_js_value(_ctx)
    }
}

impl<'js, E: JsEngine> JsException<'js, E>
where
    E::Value: JsValueImpl + JsTypeOf,
{
    pub fn from_object(value: JsObject<'js, E>) -> Option<Self> {
        if value.is_exception() {
            return Some(Self(value));
        }
        None
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsException<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_exception() {
            Ok(Self(JsObject::from_js_value(ctx, value)?))
        } else {
            Err(HostError::not_exception().into())
        }
    }
}

impl<'js, E: JsEngine> JsException<'js, E>
where
    E::Value: JsObjectOps,
{
    /// Returns the message of the error.
    pub fn message(&self) -> Option<String> {
        self.get("message").ok()
    }

    /// Returns the stack of the error.
    pub fn stack(&self) -> Option<String> {
        self.get("stack").ok()
    }

    /// Convert the exception into JSError
    pub fn into_error(self) -> JSError {
        let ctx = self.context();
        if self.is_error() {
            JSError {
                message: self.message(),
                stack: self.stack(),
            }
        } else {
            let js_value = self.as_js_value().clone();
            JSError {
                message: String::from_js_value(ctx, js_value).ok(),
                stack: None,
            }
        }
    }
}

/// Enters the JavaScript exception channel by throwing a value.
pub trait JsExceptionThrower: crate::context::JsContextImpl {
    fn throw(&self, value: Self::Value) -> Self::Value;
}

impl<'js, E: JsEngine> JsContext<'js, E>
where
    E::Context: JsExceptionThrower,
    E::Value: JsValueImpl,
{
    pub fn throw(self, value: JsValue<'js, E>) -> JsValue<'js, E> {
        let raw = self.native_context().throw(value.into_inner());
        JsValue::from_raw(self, raw)
    }
}

impl<'js, E: JsEngine> JsContext<'js, E>
where
    E::Context: JsExceptionThrower + JsErrorFactory,
    E::Value: JsValueImpl,
{
    pub fn throw_named_error(
        self,
        name: &str,
        message: impl AsRef<str>,
        code: Option<&str>,
    ) -> JsValue<'js, E> {
        let n = self.native_context();
        let built = n.new_error(name, message, code);
        let raw = n.throw(built);
        JsValue::from_raw(self, raw)
    }

    pub fn throw_syntax_error(self, message: impl AsRef<str>) -> JsValue<'js, E> {
        self.throw_named_error("SyntaxError", message, None)
    }

    pub fn throw_type_error(self, message: impl AsRef<str>) -> JsValue<'js, E> {
        self.throw_named_error("TypeError", message, None)
    }

    pub fn throw_reference_error(self, message: impl AsRef<str>) -> JsValue<'js, E> {
        self.throw_named_error("ReferenceError", message, None)
    }

    pub fn throw_range_error(self, message: impl AsRef<str>) -> JsValue<'js, E> {
        self.throw_named_error("RangeError", message, None)
    }

    pub fn throw_error(self, message: impl AsRef<str>) -> JsValue<'js, E> {
        self.throw_named_error("Error", message, None)
    }
}

impl<'js, E: JsEngine> fmt::Debug for JsException<'js, E>
where
    E::Value: JsObjectOps,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Exception")
            .field("message", &self.message())
            .field("stack", &self.stack())
            .finish()
    }
}

impl<'js, E: JsEngine> fmt::Display for JsException<'js, E>
where
    E::Value: JsObjectOps,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_error() {
            "Error:".fmt(f)?;
            if let Some(message) = &self.message() {
                ' '.fmt(f)?;
                message.fmt(f)?;
            }
            if let Some(stack) = &self.stack() {
                '\n'.fmt(f)?;
                stack.fmt(f)?;
            }
        } else {
            let ctx = self.context();
            let value = self.as_js_value().clone();
            String::from_js_value(ctx, value).unwrap().fmt(f)?;
        }
        Ok(())
    }
}

impl<'js, E: JsEngine> crate::function::JsParameterType for JsException<'js, E> where E: JsEngine {}
