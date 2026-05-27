use std::marker::PhantomData;

use hermes_sys::*;

use crate::error::{Error, Result};
use crate::{Array, ArrayBuffer, BigInt, Function, JsString, Object, Symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Undefined,
    Null,
    Boolean,
    Number,
    Symbol,
    BigInt,
    String,
    Object,
}

impl ValueKind {
    pub(crate) fn from_raw(kind: i32) -> Self {
        #[allow(non_snake_case)]
        match kind {
            HermesValueKind_Null => ValueKind::Null,
            HermesValueKind_Boolean => ValueKind::Boolean,
            HermesValueKind_Number => ValueKind::Number,
            HermesValueKind_Symbol => ValueKind::Symbol,
            HermesValueKind_BigInt => ValueKind::BigInt,
            HermesValueKind_String => ValueKind::String,
            HermesValueKind_Object => ValueKind::Object,
            _ => ValueKind::Undefined,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ValueKind::Undefined => "undefined",
            ValueKind::Null => "null",
            ValueKind::Boolean => "boolean",
            ValueKind::Number => "number",
            ValueKind::Symbol => "symbol",
            ValueKind::BigInt => "bigint",
            ValueKind::String => "string",
            ValueKind::Object => "object",
        }
    }
}

#[allow(non_snake_case)]
pub(crate) fn is_pointer_kind(kind: i32) -> bool {
    matches!(
        kind,
        HermesValueKind_String
            | HermesValueKind_Object
            | HermesValueKind_Symbol
            | HermesValueKind_BigInt
    )
}

pub struct Value<'rt> {
    pub(crate) raw: HermesValue,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Value<'rt> {
    pub fn undefined() -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Undefined,
                data: HermesValueData { number: 0.0 },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    pub fn null() -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Null,
                data: HermesValueData { number: 0.0 },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    pub fn from_bool(v: bool) -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Boolean,
                data: HermesValueData { boolean: v },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    pub fn from_number(v: f64) -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Number,
                data: HermesValueData { number: v },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    pub(crate) unsafe fn from_raw(rt: *mut HermesRt, raw: HermesValue) -> Self {
        Value {
            raw,
            rt,
            _marker: PhantomData,
        }
    }

    pub unsafe fn from_raw_clone(rt: *mut HermesRt, raw: &HermesValue) -> Self {
        unsafe {
            if is_pointer_kind(raw.kind) {
                let cloned = hermes__Value__Clone(rt, raw);
                Value {
                    raw: cloned,
                    rt,
                    _marker: PhantomData,
                }
            } else {
                Value {
                    raw: *raw,
                    rt,
                    _marker: PhantomData,
                }
            }
        }
    }

    pub fn kind(&self) -> ValueKind {
        ValueKind::from_raw(self.raw.kind)
    }

    pub fn is_undefined(&self) -> bool {
        self.raw.kind == HermesValueKind_Undefined
    }
    pub fn is_null(&self) -> bool {
        self.raw.kind == HermesValueKind_Null
    }
    pub fn is_boolean(&self) -> bool {
        self.raw.kind == HermesValueKind_Boolean
    }
    pub fn is_number(&self) -> bool {
        self.raw.kind == HermesValueKind_Number
    }
    pub fn is_symbol(&self) -> bool {
        self.raw.kind == HermesValueKind_Symbol
    }
    pub fn is_bigint(&self) -> bool {
        self.raw.kind == HermesValueKind_BigInt
    }
    pub fn is_string(&self) -> bool {
        self.raw.kind == HermesValueKind_String
    }
    pub fn is_object(&self) -> bool {
        self.raw.kind == HermesValueKind_Object
    }

    pub fn as_bool(&self) -> Option<bool> {
        if self.is_boolean() {
            Some(unsafe { self.raw.data.boolean })
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if self.is_number() {
            Some(unsafe { self.raw.data.number })
        } else {
            None
        }
    }

    pub fn into_string(self) -> Result<JsString<'rt>> {
        if !self.is_string() {
            return Err(Error::TypeError {
                expected: "string",
                got: self.kind().name(),
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(JsString {
            pv: unsafe { this.raw.data.pointer },
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn into_object(self) -> Result<Object<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "object",
                got: self.kind().name(),
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(Object {
            pv: unsafe { this.raw.data.pointer },
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn into_function(self) -> Result<Function<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "function",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let is_fn = unsafe { hermes__Object__IsFunction(self.rt, ptr) };
        if !is_fn {
            return Err(Error::TypeError {
                expected: "function",
                got: "object",
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(Function {
            pv: ptr,
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn into_array(self) -> Result<Array<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "array",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let is_arr = unsafe { hermes__Object__IsArray(self.rt, ptr) };
        if !is_arr {
            return Err(Error::TypeError {
                expected: "array",
                got: "object",
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(Array {
            pv: ptr,
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn into_symbol(self) -> Result<Symbol<'rt>> {
        if !self.is_symbol() {
            return Err(Error::TypeError {
                expected: "symbol",
                got: self.kind().name(),
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(Symbol {
            pv: unsafe { this.raw.data.pointer },
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn into_bigint(self) -> Result<BigInt<'rt>> {
        if !self.is_bigint() {
            return Err(Error::TypeError {
                expected: "bigint",
                got: self.kind().name(),
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(BigInt {
            pv: unsafe { this.raw.data.pointer },
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn into_array_buffer(self) -> Result<ArrayBuffer<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "arraybuffer",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let is_ab = unsafe { hermes__Object__IsArrayBuffer(self.rt, ptr) };
        if !is_ab {
            return Err(Error::TypeError {
                expected: "arraybuffer",
                got: "object",
            });
        }
        let this = std::mem::ManuallyDrop::new(self);
        Ok(ArrayBuffer {
            pv: ptr,
            rt: this.rt,
            _marker: PhantomData,
        })
    }

    pub fn to_js_string(&self) -> Result<JsString<'rt>> {
        let pv = unsafe { hermes__Value__ToString(self.rt, &self.raw) };
        crate::error::check_error(self.rt)?;
        Ok(JsString {
            pv,
            rt: self.rt,
            _marker: PhantomData,
        })
    }

    pub fn duplicate(&self) -> Value<'rt> {
        if is_pointer_kind(self.raw.kind) {
            let raw = unsafe { hermes__Value__Clone(self.rt, &self.raw) };
            Value {
                raw,
                rt: self.rt,
                _marker: PhantomData,
            }
        } else {
            Value {
                raw: self.raw,
                rt: self.rt,
                _marker: PhantomData,
            }
        }
    }

    pub fn into_raw(self) -> HermesValue {
        let this = std::mem::ManuallyDrop::new(self);
        this.raw
    }

    pub fn strict_equals(&self, other: &Value<'rt>) -> bool {
        unsafe { hermes__Value__StrictEquals(self.rt, &self.raw, &other.raw) }
    }

    pub fn unique_id(&self) -> u64 {
        unsafe { hermes__Value__GetUniqueID(self.rt, &self.raw) }
    }
}

impl Drop for Value<'_> {
    fn drop(&mut self) {
        if is_pointer_kind(self.raw.kind) {
            unsafe { hermes__Value__Release(&mut self.raw) };
        }
    }
}

impl std::fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind() {
            ValueKind::Undefined => write!(f, "Value(undefined)"),
            ValueKind::Null => write!(f, "Value(null)"),
            ValueKind::Boolean => write!(f, "Value({})", self.as_bool().unwrap()),
            ValueKind::Number => write!(f, "Value({})", self.as_number().unwrap()),
            ValueKind::String => write!(f, "Value(string)"),
            ValueKind::Object => write!(f, "Value(object)"),
            ValueKind::Symbol => write!(f, "Value(symbol)"),
            ValueKind::BigInt => write!(f, "Value(bigint)"),
        }
    }
}
