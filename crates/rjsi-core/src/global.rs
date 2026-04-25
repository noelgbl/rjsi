//! Explicit rooted handles for values that escape an active scope.

use crate::JsEngine;

pub trait JsGlobalHandle<E: JsEngine>: Clone + 'static {
    fn new<'js>(scope: &mut E::Scope<'js>, value: &E::Value<'js>) -> Self;
    fn get<'js>(&self, scope: &mut E::Scope<'js>) -> E::Value<'js>;
}

pub struct Global<E: JsEngine> {
    handle: E::Global,
}

impl<E: JsEngine> Global<E> {
    pub fn new<'js>(scope: &mut E::Scope<'js>, value: &E::Value<'js>) -> Self {
        Self {
            handle: E::Global::new(scope, value),
        }
    }

    pub fn get<'js>(&self, scope: &mut E::Scope<'js>) -> E::Value<'js> {
        self.handle.get(scope)
    }

    pub fn handle(&self) -> &E::Global {
        &self.handle
    }
}

impl<E: JsEngine> Clone for Global<E> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}
