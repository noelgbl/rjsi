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

pub trait NativeStateSupport: Engine {
    fn object_create_with_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        state: S,
    ) -> Result<Object<'js, Self>>;

    fn object_get_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &Object<'js, Self>,
    ) -> Option<&'js S>;

    fn object_get_state_mut<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &mut Object<'js, Self>,
    ) -> Option<&'js mut S>;
}

pub trait ContextNativeStateExt<'js, E: NativeStateSupport> {
    fn with_state<S: NativeState>(&mut self, state: S) -> Result<Object<'js, E>>;
    fn get_state<S: NativeState>(&mut self, obj: &Object<'js, E>) -> Option<&'js S>;
    fn get_state_mut<S: NativeState>(&mut self, obj: &mut Object<'js, E>) -> Option<&'js mut S>;
}

impl<'js, E: NativeStateSupport> ContextNativeStateExt<'js, E> for Context<'js, E> {
    fn with_state<S: NativeState>(&mut self, state: S) -> Result<Object<'js, E>> {
        E::object_create_with_state(self, state)
    }

    fn get_state<S: NativeState>(&mut self, obj: &Object<'js, E>) -> Option<&'js S> {
        E::object_get_state(self, obj)
    }

    fn get_state_mut<S: NativeState>(&mut self, obj: &mut Object<'js, E>) -> Option<&'js mut S> {
        E::object_get_state_mut(self, obj)
    }
}
