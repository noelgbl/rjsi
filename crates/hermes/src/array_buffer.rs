use std::marker::PhantomData;

use hermes_sys::*;

use crate::Runtime;
use crate::error::Error;
use crate::value::Value;

pub struct ArrayBuffer<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> ArrayBuffer<'rt> {
    pub fn new(rt: &'rt Runtime, size: usize) -> Self {
        let pv = unsafe { hermes__ArrayBuffer__New(rt.raw, size) };
        ArrayBuffer {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    pub fn size(&self) -> usize {
        unsafe { hermes__ArrayBuffer__Size(self.rt, self.pv) }
    }

    pub fn data(&self) -> &[u8] {
        let ptr = unsafe { hermes__ArrayBuffer__Data(self.rt, self.pv) };
        let len = self.size();
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        let ptr = unsafe { hermes__ArrayBuffer__Data(self.rt, self.pv) };
        let len = self.size();
        if ptr.is_null() || len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(ptr, len) }
        }
    }
}

impl Drop for ArrayBuffer<'_> {
    fn drop(&mut self) {
        unsafe { hermes__Object__Release(self.pv) }
    }
}

impl<'rt> From<ArrayBuffer<'rt>> for Value<'rt> {
    fn from(buf: ArrayBuffer<'rt>) -> Value<'rt> {
        let buf = std::mem::ManuallyDrop::new(buf);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: buf.pv },
            },
            rt: buf.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for ArrayBuffer<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> crate::error::Result<Self> {
        val.into_array_buffer()
    }
}

impl std::fmt::Debug for ArrayBuffer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArrayBuffer(size={})", self.size())
    }
}
