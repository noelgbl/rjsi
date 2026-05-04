use std::marker::PhantomData;

use crate::{Args, CallbackCx, Context, Engine, Function, JsResult, Object};

pub trait JsClass<E: Engine>: Sized + 'static {
    const NAME: &'static str;

    fn define_prototype<'cx>(
        cx: &mut Context<'cx, E>,
        proto: &Object<'cx, E>,
    ) -> JsResult<'cx, E, ()>
    where
        E: ClassEngine;

    fn construct<'cx, 'rt>(
        cx: &mut CallbackCx<'cx, 'rt, E>,
        args: Args<'rt, E>,
    ) -> JsResult<'rt, E, Self>
    where
        E: ClassEngine;
}

pub trait ClassEngine: Engine {
    fn class_register<'rt, C: JsClass<Self>>(
        cx: &mut Context<'rt, Self>,
    ) -> JsResult<'rt, Self, Function<'rt, Self>>;
    
    unsafe fn class_get_instance_ptr<C: 'static>(
        cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C>;
}

pub struct InstanceRef<'cx, C: 'static> {
    ptr: *mut C,
    _phantom: PhantomData<&'cx mut C>,
}

impl<'cx, C: 'static> InstanceRef<'cx, C> {
    pub unsafe fn from_raw(ptr: *mut C) -> Self {
        Self { ptr, _phantom: PhantomData }
    }

    pub fn get(&self) -> &C {
        unsafe { &*self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut C {
        unsafe { &mut *self.ptr }
    }
}

pub trait ContextClassExt<'rt, E: Engine + ClassEngine> {
    fn register_class<C: JsClass<E>>(&mut self) -> JsResult<'rt, E, Function<'rt, E>>;
}

impl<'rt, E: Engine + ClassEngine> ContextClassExt<'rt, E> for Context<'rt, E> {
    fn register_class<C: JsClass<E>>(&mut self) -> JsResult<'rt, E, Function<'rt, E>> {
        E::class_register::<C>(self)
    }
}
