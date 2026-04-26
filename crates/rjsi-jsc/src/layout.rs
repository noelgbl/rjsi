use rusty_jsc::JSObject;
use rusty_jsc_sys::JSObjectRef;

#[inline]
pub(crate) fn jsobject_ref<T>(o: &JSObject<T>) -> JSObjectRef {
    unsafe { std::ptr::read(std::ptr::from_ref(o).cast::<JSObjectRef>()) }
}
