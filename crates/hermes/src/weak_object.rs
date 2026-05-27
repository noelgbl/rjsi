use std::marker::PhantomData;

use hermes_sys::*;

use crate::error::{Result, check_error};
use crate::value::Value;
use crate::{Object, Runtime};

pub struct WeakObject<'rt> {
    pv: *mut std::ffi::c_void,
    rt: *mut HermesRt,
    _marker: PhantomData<&'rt ()>,
}

impl<'rt> WeakObject<'rt> {
    pub fn new(rt: &'rt Runtime, obj: &Object<'rt>) -> Self {
        let pv = unsafe { hermes__WeakObject__Create(rt.raw, obj.pv) };
        WeakObject {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    pub fn lock(&self) -> Result<Option<Value<'rt>>> {
        let raw = unsafe { hermes__WeakObject__Lock(self.rt, self.pv) };
        check_error(self.rt)?;
        if raw.kind == HermesValueKind_Undefined {
            Ok(None)
        } else {
            Ok(Some(unsafe { Value::from_raw(self.rt, raw) }))
        }
    }
}

impl Drop for WeakObject<'_> {
    fn drop(&mut self) {
        unsafe { hermes__WeakObject__Release(self.pv) }
    }
}

impl std::fmt::Debug for WeakObject<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WeakObject({:?})", self.pv)
    }
}
