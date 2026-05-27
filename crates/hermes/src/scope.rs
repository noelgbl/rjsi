use hermes_sys::*;

use crate::Runtime;

pub struct Scope {
    raw: *mut std::ffi::c_void,
}

impl Scope {
    pub fn new(rt: &Runtime) -> Self {
        let raw = unsafe { hermes__Scope__New(rt.raw) };
        Scope { raw }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        unsafe { hermes__Scope__Delete(self.raw) }
    }
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scope({:?})", self.raw)
    }
}
