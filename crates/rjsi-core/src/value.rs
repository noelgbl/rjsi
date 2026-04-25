use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JsValueType {
    Undefined,
    Null,
    Boolean,
    Number,
    BigInt,
    String,
    Symbol,
    Object,
    Array,
    ArrayBuffer,
    Function,
    Promise,
    Error,
    Exception,
    Date,
    Unknown,
}

impl fmt::Display for JsValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            JsValueType::Undefined => "undefined",
            JsValueType::Null => "null",
            JsValueType::Boolean => "boolean",
            JsValueType::Number => "number",
            JsValueType::BigInt => "bigint",
            JsValueType::String => "string",
            JsValueType::Symbol => "symbol",
            JsValueType::Object => "object",
            JsValueType::Array => "array",
            JsValueType::ArrayBuffer => "arrayBuffer",
            JsValueType::Function => "function",
            JsValueType::Promise => "promise",
            JsValueType::Error => "error",
            JsValueType::Exception => "exception",
            JsValueType::Date => "date",
            JsValueType::Unknown => "unknown",
        })
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PropertyAttributes(u32);

impl PropertyAttributes {
    const WRITABLE: u32 = 1;
    const ENUMERABLE: u32 = 1 << 1;
    const CONFIGURABLE: u32 = 1 << 2;

    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn writable(mut self) -> Self {
        self.0 |= Self::WRITABLE;
        self
    }

    #[must_use]
    pub const fn enumerable(mut self) -> Self {
        self.0 |= Self::ENUMERABLE;
        self
    }

    #[must_use]
    pub const fn configurable(mut self) -> Self {
        self.0 |= Self::CONFIGURABLE;
        self
    }

    #[must_use]
    pub const fn is_writable(self) -> bool {
        self.0 & Self::WRITABLE != 0
    }

    #[must_use]
    pub const fn is_enumerable(self) -> bool {
        self.0 & Self::ENUMERABLE != 0
    }

    #[must_use]
    pub const fn is_configurable(self) -> bool {
        self.0 & Self::CONFIGURABLE != 0
    }
}
