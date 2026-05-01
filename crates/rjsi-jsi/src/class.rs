use std::cell::RefCell;
use std::marker::PhantomData;

use crate::{Args, CallbackCx, Context, Engine, JsResult};

#[derive(Clone, Copy)]
pub struct NativePtr {
    ptr: *mut (),
}

impl NativePtr {
    pub const NULL: Self = Self {
        ptr: std::ptr::null_mut(),
    };

    pub fn new<T>(p: *mut T) -> Self {
        Self {
            ptr: p.cast::<()>(),
        }
    }

    pub fn as_ptr(self) -> *mut () {
        self.ptr
    }

    pub fn is_null(self) -> bool {
        self.ptr.is_null()
    }
}

pub trait NativeObject<E: Engine> {
    fn native_ptr(&self) -> NativePtr;
    fn set_native_ptr(&mut self, ptr: NativePtr);
}

pub trait JsClass<E: Engine>: 'static {
    const NAME: &'static str;

    fn prototype<'cx, 'rt>(cx: &mut Context<'rt, E>, proto: E::Object<'cx>)
    -> JsResult<'cx, E, ()>;

    fn constructor<'cx, 'rt>(
        cx: &mut CallbackCx<'cx, 'rt, E>,
        args: Args<'cx, E>,
    ) -> JsResult<'cx, E, Self>
    where
        Self: Sized;
}

pub struct NativeCell<T> {
    inner: RefCell<Option<T>>,
}

impl<T: 'static> NativeCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(Some(value)),
        }
    }

    pub fn try_with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> Result<R, NativeBorrowError> {
        let mut slot = self
            .inner
            .try_borrow_mut()
            .map_err(|_| NativeBorrowError::AlreadyBorrowed)?;
        let Some(t) = slot.as_mut() else {
            return Err(NativeBorrowError::Empty);
        };
        Ok(f(t))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeBorrowError {
    AlreadyBorrowed,
    Empty,
}

pub struct NativeBorrowGuard<'a, T> {
    _marker: PhantomData<&'a mut T>,
}
