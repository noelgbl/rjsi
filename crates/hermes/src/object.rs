use std::marker::PhantomData;

use hermes_sys::*;

use crate::error::{Error, Result, check_error};
use crate::propnameid::PropNameId;
use crate::value::Value;
use crate::{Array, Runtime};

pub struct Object<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Object<'rt> {
    pub fn new(rt: &'rt Runtime) -> Self {
        let pv = unsafe { hermes__Object__New(rt.raw) };
        Object {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    pub fn get(&self, key: &str) -> Result<Value<'rt>> {
        let key_pv = unsafe { hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len()) };
        let raw = unsafe { hermes__Object__GetProperty__String(self.rt, self.pv, key_pv) };
        unsafe { hermes__String__Release(key_pv) };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    pub fn set(&self, key: &str, val: Value<'rt>) -> Result<()> {
        let key_pv = unsafe { hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len()) };
        let ok = unsafe { hermes__Object__SetProperty__String(self.rt, self.pv, key_pv, &val.raw) };
        unsafe { hermes__String__Release(key_pv) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn has(&self, key: &str) -> bool {
        let key_pv = unsafe { hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len()) };
        let result = unsafe { hermes__Object__HasProperty__String(self.rt, self.pv, key_pv) };
        unsafe { hermes__String__Release(key_pv) };
        result
    }

    pub fn get_with_propname(&self, key: &PropNameId<'rt>) -> Result<Value<'rt>> {
        let raw = unsafe { hermes__Object__GetProperty__PropNameID(self.rt, self.pv, key.pv) };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    pub fn set_with_propname(&self, key: &PropNameId<'rt>, val: Value<'rt>) -> Result<()> {
        let ok =
            unsafe { hermes__Object__SetProperty__PropNameID(self.rt, self.pv, key.pv, &val.raw) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn has_with_propname(&self, key: &PropNameId<'rt>) -> bool {
        unsafe { hermes__Object__HasProperty__PropNameID(self.rt, self.pv, key.pv) }
    }

    pub unsafe fn create_host_object(
        rt: &'rt Runtime,
        get_cb: HermesHostObjectGetCallback,
        set_cb: HermesHostObjectSetCallback,
        get_names_cb: HermesHostObjectGetPropertyNamesCallback,
        user_data: *mut std::ffi::c_void,
        finalizer: HermesHostObjectFinalizer,
    ) -> Self {
        unsafe {
            let pv = hermes__Object__CreateFromHostObject(
                rt.raw,
                get_cb,
                set_cb,
                get_names_cb,
                user_data,
                finalizer,
            );
            Object {
                pv,
                rt: rt.raw,
                _marker: PhantomData,
            }
        }
    }

    pub fn get_host_object_data(&self) -> *mut std::ffi::c_void {
        unsafe { hermes__Object__GetHostObject(self.rt, self.pv) }
    }

    pub fn property_names(&self) -> Result<Array<'rt>> {
        let arr_pv = unsafe { hermes__Object__GetPropertyNames(self.rt, self.pv) };
        check_error(self.rt)?;
        Ok(Array {
            pv: arr_pv,
            rt: self.rt,
            _marker: PhantomData,
        })
    }

    pub fn is_array(&self) -> bool {
        unsafe { hermes__Object__IsArray(self.rt, self.pv) }
    }

    pub fn is_function(&self) -> bool {
        unsafe { hermes__Object__IsFunction(self.rt, self.pv) }
    }

    pub fn is_array_buffer(&self) -> bool {
        unsafe { hermes__Object__IsArrayBuffer(self.rt, self.pv) }
    }

    pub fn strict_equals(&self, other: &Object<'rt>) -> bool {
        unsafe { hermes__Object__StrictEquals(self.rt, self.pv, other.pv) }
    }

    pub fn instance_of(&self, func: &Object<'rt>) -> bool {
        unsafe { hermes__Object__InstanceOf(self.rt, self.pv, func.pv) }
    }

    pub fn set_external_memory_pressure(&self, amount: usize) {
        unsafe { hermes__Object__SetExternalMemoryPressure(self.rt, self.pv, amount) }
    }

    pub fn has_native_state(&self) -> bool {
        unsafe { hermes__Object__HasNativeState(self.rt, self.pv) }
    }

    pub fn get_native_state(&self) -> *mut std::ffi::c_void {
        unsafe { hermes__Object__GetNativeState(self.rt, self.pv) }
    }

    pub unsafe fn set_native_state(
        &self,
        data: *mut std::ffi::c_void,
        finalizer: HermesNativeStateFinalizer,
    ) {
        unsafe {
            hermes__Object__SetNativeState(self.rt, self.pv, data, finalizer);
        }
    }

    pub fn is_host_object(&self) -> bool {
        unsafe { hermes__Object__IsHostObject(self.rt, self.pv) }
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let key_pv = unsafe { hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len()) };
        let ok = unsafe { hermes__Object__DeleteProperty__String(self.rt, self.pv, key_pv) };
        unsafe { hermes__String__Release(key_pv) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn delete_with_propname(&self, key: &PropNameId<'rt>) -> Result<()> {
        let ok = unsafe { hermes__Object__DeleteProperty__PropNameID(self.rt, self.pv, key.pv) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn delete_with_value(&self, key: &Value<'rt>) -> Result<()> {
        let ok = unsafe { hermes__Object__DeleteProperty__Value(self.rt, self.pv, &key.raw) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn get_with_value(&self, key: &Value<'rt>) -> Result<Value<'rt>> {
        let raw = unsafe { hermes__Object__GetProperty__Value(self.rt, self.pv, &key.raw) };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    pub fn set_with_value(&self, key: &Value<'rt>, val: Value<'rt>) -> Result<()> {
        let ok =
            unsafe { hermes__Object__SetProperty__Value(self.rt, self.pv, &key.raw, &val.raw) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn has_with_value(&self, key: &Value<'rt>) -> bool {
        unsafe { hermes__Object__HasProperty__Value(self.rt, self.pv, &key.raw) }
    }

    pub fn create_with_prototype(rt: &'rt Runtime, prototype: &Value<'rt>) -> Result<Self> {
        let pv = unsafe { hermes__Object__CreateWithPrototype(rt.raw, &prototype.raw) };
        check_error(rt.raw)?;
        Ok(Object {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        })
    }

    pub fn set_prototype(&self, prototype: &Value<'rt>) -> Result<()> {
        let ok = unsafe { hermes__Object__SetPrototype(self.rt, self.pv, &prototype.raw) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    pub fn get_prototype(&self) -> Result<Value<'rt>> {
        let raw = unsafe { hermes__Object__GetPrototype(self.rt, self.pv) };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    pub fn unique_id(&self) -> u64 {
        unsafe { hermes__Object__GetUniqueID(self.rt, self.pv) }
    }
}

impl Drop for Object<'_> {
    fn drop(&mut self) {
        unsafe { hermes__Object__Release(self.pv) }
    }
}

impl<'rt> From<Object<'rt>> for Value<'rt> {
    fn from(obj: Object<'rt>) -> Value<'rt> {
        let obj = std::mem::ManuallyDrop::new(obj);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: obj.pv },
            },
            rt: obj.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for Object<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> Result<Self> {
        val.into_object()
    }
}

impl std::fmt::Debug for Object<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Object({:?})", self.pv)
    }
}
