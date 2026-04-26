use rjsi_core::PersistentLike;
use v8 as rv8;

use crate::runtime::{V8Runtime, V8Scope};

#[derive(Clone, Copy)]
pub struct V8Value<'js> {
    pub(crate) local: rv8::Local<'js, rv8::Value>,
}

impl std::fmt::Debug for V8Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("V8Value").finish_non_exhaustive()
    }
}

impl<'js> V8Value<'js> {
    pub(crate) fn from_local<T>(local: rv8::Local<'js, T>) -> Self
    where
        rv8::Local<'js, T>: Into<rv8::Local<'js, rv8::Value>>,
    {
        Self {
            local: local.into(),
        }
    }
}

#[derive(Clone)]
pub struct V8Global {
    handle: rv8::Global<rv8::Value>,
}

impl PersistentLike<V8Runtime> for V8Global {
    fn new<'s, 'p: 's>(scope: &mut V8Scope<'s, 'p>, value: V8Value<'s>) -> Self {
        Self {
            handle: {
                let active_scope = unsafe { &mut *scope.scope };
                let isolate = active_scope.as_ref();
                rv8::Global::new(isolate, value.local)
            },
        }
    }

    fn get<'s, 'p: 's>(&self, scope: &mut V8Scope<'s, 'p>) -> V8Value<'s> {
        let local = rv8::Local::new(scope.scope(), &self.handle);
        V8Value { local }
    }
}
