use std::cell::RefCell;
use std::marker::PhantomData;

use crate::{Args, CallbackCx, Context, Engine, JsResult, Object, Value};

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

    fn prototype<'cx>(
        cx: &mut Context<'cx, E>,
        proto: &Object<'cx, E>,
    ) -> JsResult<'cx, E, ()>;

    fn constructor<'cx, 'rt>(
        cx: &mut CallbackCx<'cx, 'rt, E>,
        args: Args<'rt, E>,
    ) -> JsResult<'rt, E, Self>
    where
        Self: Sized;
}

pub type ClassMethodFn<E> = for<'cx, 'rt> fn(
    &mut CallbackCx<'cx, 'rt, E>,
    Value<'rt, E>,
    Args<'rt, E>,
) -> JsResult<'rt, E, Value<'rt, E>>;

pub struct ClassMethod<E: Engine> {
    pub name: &'static str,
    pub func: ClassMethodFn<E>,
}

pub struct ClassAccessor<E: Engine> {
    pub name: &'static str,
    pub get: Option<ClassMethodFn<E>>,
    pub set: Option<ClassMethodFn<E>>,
}

pub struct ClassDescriptor<E: Engine> {
    pub name: &'static str,
    pub methods: &'static [ClassMethod<E>],
    pub statics: &'static [ClassMethod<E>],
    pub accessors: &'static [ClassAccessor<E>],
}

impl<E: Engine> ClassDescriptor<E> {
    pub const fn empty(name: &'static str) -> Self {
        Self {
            name,
            methods: &[],
            statics: &[],
            accessors: &[],
        }
    }
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
