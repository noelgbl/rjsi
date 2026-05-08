use std::any::{Any, TypeId};
use std::ffi::c_void;

use crate::{Context, Engine, Object, Result};

#[repr(C)]
pub struct TaggedNativeState<S> {
    pub type_id: TypeId,
    pub value: S,
}

impl<S: 'static> TaggedNativeState<S> {
    pub fn new(value: S) -> Self {
        Self {
            type_id: TypeId::of::<S>(),
            value,
        }
    }
}

#[inline]
pub unsafe fn tagged_native_state_type_id(ptr: *mut c_void) -> TypeId {
    unsafe { std::ptr::read(ptr as *const TypeId) }
}

pub struct ErasedNativeState {
    pub inner: Box<dyn Any>,
}

pub trait NativeState: Any + 'static {}

pub trait NativeStateEngine: Engine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>>;

    fn object_get_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S>;

    fn object_get_state_mut<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &mut Object<'cx, Self>,
    ) -> Option<&'cx mut S>;
}

pub trait ContextNativeStateExt<'rt, E: NativeStateEngine> {
    fn create_with_state<S: NativeState>(&mut self, state: S) -> Result<Object<'rt, E>>;
    fn get_state<S: NativeState>(&mut self, obj: &Object<'rt, E>) -> Option<&'rt S>;
    fn get_state_mut<S: NativeState>(&mut self, obj: &mut Object<'rt, E>) -> Option<&'rt mut S>;
}

impl<'rt, E: NativeStateEngine> ContextNativeStateExt<'rt, E> for Context<'rt, E> {
    fn create_with_state<S: NativeState>(&mut self, state: S) -> Result<Object<'rt, E>> {
        E::object_create_with_state(self, state)
    }

    fn get_state<S: NativeState>(&mut self, obj: &Object<'rt, E>) -> Option<&'rt S> {
        E::object_get_state(self, obj)
    }

    fn get_state_mut<S: NativeState>(&mut self, obj: &mut Object<'rt, E>) -> Option<&'rt mut S> {
        E::object_get_state_mut(self, obj)
    }
}
