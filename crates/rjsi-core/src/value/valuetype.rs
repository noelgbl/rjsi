use crate::{JsEngine, JsValue, JsValueImpl};
use std::fmt;

#[derive(Clone, Debug)]
pub enum JsValueType {
    Undefined,
    Null,
    Error,
    Exception,
    Boolean,
    Number,
    BigInt,
    String,
    Object,
    Array,
    ArrayBuffer,
    Function,
    Constructor,
    Promise,
    Symbol,
    Date,
    Unknown,
}

impl fmt::Display for JsValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JsValueType::Undefined => "undefined",
            JsValueType::Null => "null",
            JsValueType::Error => "error",
            JsValueType::Exception => "exception",
            JsValueType::Boolean => "boolean",
            JsValueType::Number => "number",
            JsValueType::BigInt => "bigint",
            JsValueType::String => "string",
            JsValueType::Object => "object",
            JsValueType::Array => "array",
            JsValueType::ArrayBuffer => "arrayBuffer",
            JsValueType::Function => "function",
            JsValueType::Constructor => "constructor",
            JsValueType::Promise => "promise",
            JsValueType::Symbol => "symbol",
            JsValueType::Date => "Date",
            JsValueType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

pub trait JsTypeOf: JsValueImpl {
    fn is_exception(&self) -> bool;
    fn is_error(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_array_buffer(&self) -> bool;
    fn is_promise(&self) -> bool;
    fn is_undefined(&self) -> bool;
    fn is_null(&self) -> bool;
    fn is_boolean(&self) -> bool;
    fn is_number(&self) -> bool;
    fn is_bigint(&self) -> bool;
    fn is_string(&self) -> bool;
    fn is_symbol(&self) -> bool;
    fn is_function(&self) -> bool;
    fn is_object(&self) -> bool;
    fn is_constructor(&self) -> bool;
    fn is_date(&self) -> bool;
    fn is_proxy(&self) -> bool;

    fn type_of(&self) -> JsValueType {
        if self.is_exception() {
            JsValueType::Exception
        } else if self.is_error() {
            JsValueType::Error
        } else if self.is_promise() {
            JsValueType::Promise
        } else if self.is_array() {
            JsValueType::Array
        } else if self.is_array_buffer() {
            JsValueType::ArrayBuffer
        } else if self.is_function() {
            JsValueType::Function
        } else if self.is_constructor() {
            JsValueType::Constructor
        } else if self.is_undefined() {
            JsValueType::Undefined
        } else if self.is_null() {
            JsValueType::Null
        } else if self.is_boolean() {
            JsValueType::Boolean
        } else if self.is_number() {
            JsValueType::Number
        } else if self.is_bigint() {
            JsValueType::BigInt
        } else if self.is_string() {
            JsValueType::String
        } else if self.is_date() {
            JsValueType::Date
        } else if self.is_symbol() {
            JsValueType::Symbol
        } else if self.is_object() {
            // check is_object at last stage, since such as function etc is also object
            JsValueType::Object
        } else {
            JsValueType::Unknown
        }
    }
}

impl<'js, E: JsEngine> JsValue<'js, E>
where
    E::Value: JsTypeOf,
{
    pub fn type_of(&self) -> JsValueType {
        self.inner.type_of()
    }
}

macro_rules! generate_is_type {
    ($($take_method: ident => $is_method: ident),*) => {
        impl<'js, E: JsEngine> JsValue<'js, E>
        where
            E::Value: JsTypeOf,
        {
            $(
                pub fn $take_method(self) -> Option<Self> {
                    if self.inner.$is_method() {
                        Some(self)
                    } else {
                        None
                    }
                }

                pub fn $is_method(&self) -> bool {
                    self.inner.$is_method()
                }
            )*
        }
    }
}

generate_is_type!(
    take_is_object => is_object,
    take_is_array => is_array,
    take_is_array_buffer => is_array_buffer,
    take_is_function => is_function,
    take_is_constructor => is_constructor,
    take_is_promise => is_promise,
    take_is_error => is_error,
    take_is_exception => is_exception,
    take_is_undefined => is_undefined,
    take_is_null => is_null,
    take_is_boolean => is_boolean,
    take_is_number => is_number,
    take_is_bigint => is_bigint,
    take_is_string => is_string,
    take_is_symbol => is_symbol,
    take_is_date => is_date,
    take_is_proxy => is_proxy
);
