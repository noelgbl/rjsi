//! Helpers for a few `rusty_jsc_sys` entry points that need an opaque `JSObjectRef`. `JSValue`
//! exposes [`rusty_jsc::JSValue::get_ref`]; `JSObject` does not expose `inner` publicly.
//!
//! **Do not** derive a `JSContextRef` from `&JSContext` by punning struct memory: use
//! [`JSContext::get_ref`](rusty_jsc::JSContext::get_ref) instead (see the main `JscScope` / runtime).

use rusty_jsc::JSObject;
use rusty_jsc_sys::JSObjectRef;

#[inline]
pub(crate) fn jsobject_ref<T>(o: &JSObject<T>) -> JSObjectRef {
    // `JSObject` is `{ inner, data: Option<...> }`; only read `inner` (first field; same in current `rusty_jsc`).
    unsafe { std::ptr::read(std::ptr::from_ref(o).cast::<JSObjectRef>()) }
}
