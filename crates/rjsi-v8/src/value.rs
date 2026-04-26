use rjsi_core::JsGlobalHandle;
use v8 as rv8;

use crate::runtime::V8Engine;

#[derive(Clone, Copy)]
pub struct V8Value<'js> {
    pub(crate) local: rv8::Local<'js, rv8::Value>,
    pub(crate) exception: bool,
}

impl std::fmt::Debug for V8Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("V8Value")
            .field("exception", &self.exception)
            .finish_non_exhaustive()
    }
}

impl<'js> V8Value<'js> {
    pub(crate) fn from_local<T>(local: rv8::Local<'js, T>, exception: bool) -> Self
    where
        rv8::Local<'js, T>: Into<rv8::Local<'js, rv8::Value>>,
    {
        Self {
            local: local.into(),
            exception,
        }
    }
}

#[derive(Clone, Copy)]
pub struct V8PropertyKey<'js> {
    pub(crate) local: rv8::Local<'js, rv8::Name>,
}

#[derive(Clone)]
pub struct V8Global {
    handle: rv8::Global<rv8::Value>,
    exception: bool,
}

impl JsGlobalHandle<V8Engine> for V8Global {
    fn new<'js>(
        scope: &mut <V8Engine as rjsi_core::JsEngine>::Scope<'js>,
        value: &<V8Engine as rjsi_core::JsEngine>::Value<'js>,
    ) -> Self {
        Self {
            handle: {
                let active_scope = unsafe { &mut *scope.scope };
                let isolate = active_scope.as_ref();
                rv8::Global::new(isolate, value.local)
            },
            exception: value.exception,
        }
    }

    fn get<'js>(
        &self,
        scope: &mut <V8Engine as rjsi_core::JsEngine>::Scope<'js>,
    ) -> <V8Engine as rjsi_core::JsEngine>::Value<'js> {
        let local = rv8::Local::new(scope.scope(), &self.handle);
        V8Value {
            local,
            exception: self.exception,
        }
    }
}
