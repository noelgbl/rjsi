use std::marker::PhantomData;
use std::ptr::NonNull;

use super::NativeClass;

/// A scope-lifetime-scoped mutable reference to the Rust value inside a JS object.
///
/// The `'s` lifetime ties this reference to the engine scope that produced it.
/// It cannot outlive the scope, preventing dangling pointer access after GC.
///
/// Internally it is just a raw pointer — the same size as `&mut T`.
/// No boxing beyond the one that was already there.
pub struct NativeRef<'s, T: NativeClass = Erased> {
    ptr: NonNull<T>,
    _scope: PhantomData<&'s mut T>,
}

/// Type-erased variant used in function pointer signatures where the concrete type is
/// established by the calling bridge code.
pub enum Erased {}

unsafe impl NativeClass for Erased {
    const NAME: &'static str = "__erased__";

    fn descriptor<R: crate::Runtime>() -> &'static crate::ClassDescriptor<R> {
        panic!("Erased::descriptor must not be called")
    }
}

impl<'s, T: NativeClass> NativeRef<'s, T> {
    /// # Safety
    /// `ptr` must point to a live `Box<T>` stored in the object's native slot,
    /// and `'s` must correctly bound the scope that owns the object.
    #[inline]
    pub unsafe fn new(ptr: *mut T) -> Self {
        unsafe {
            Self {
                ptr: NonNull::new_unchecked(ptr),
                _scope: PhantomData,
            }
        }
    }

    /// Type-erasing cast — used when passing to [`super::descriptor::InstanceMethodFn`].
    #[inline]
    pub fn erase(self) -> NativeRef<'s, Erased> {
        NativeRef {
            ptr: self.ptr.cast(),
            _scope: PhantomData,
        }
    }

    #[inline]
    pub fn get(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}
