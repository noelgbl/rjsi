use std::marker::PhantomData;

use crate::{Args, Context, Engine, Function, Object, Result};

pub trait JsClass<E: Engine>: Sized + 'static {
    const NAME: &'static str;

    fn define_prototype<'js>(cx: &mut Context<'js, E>, proto: &Object<'js, E>) -> Result<()>
    where
        E: ClassSupport;

    fn construct<'js>(cx: &mut Context<'js, E>, args: Args<'js, E>) -> Result<Self>
    where
        E: ClassSupport;
}

pub trait ClassSupport: Engine {
    fn class_register<'js, C: JsClass<Self>>(
        cx: &mut Context<'js, Self>,
    ) -> Result<Function<'js, Self>>;

    unsafe fn class_get_instance_ptr<C: 'static>(
        cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C>;
}

pub struct InstanceRef<'js, C: 'static> {
    ptr: *mut C,
    _phantom: PhantomData<&'js mut C>,
}

impl<'js, C: 'static> InstanceRef<'js, C> {
    pub unsafe fn from_raw(ptr: *mut C) -> Self {
        Self {
            ptr,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self) -> &C {
        unsafe { &*self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut C {
        unsafe { &mut *self.ptr }
    }
}

pub trait ContextClassExt<'js, E: Engine + ClassSupport> {
    fn register_class<C: JsClass<E>>(&mut self) -> Result<Function<'js, E>>;
}

impl<'js, E: Engine + ClassSupport> ContextClassExt<'js, E> for Context<'js, E> {
    fn register_class<C: JsClass<E>>(&mut self) -> Result<Function<'js, E>> {
        E::class_register::<C>(self)
    }
}
