use std::fmt;

use crate::{Context, Engine, Object, Value};

#[repr(transparent)]
pub struct JsException<'js, E: Engine>(pub(crate) Object<'js, E>);

impl<'js, E: Engine> JsException<'js, E> {
    pub fn from_object(obj: Object<'js, E>) -> Self {
        Self(obj)
    }

    pub fn into_object(self) -> Object<'js, E> {
        self.0
    }

    pub fn as_object(&self) -> &Object<'js, E> {
        &self.0
    }

    pub fn into_value(self) -> Value<'js, E> {
        self.0.into_value()
    }

    pub fn name(&self, cx: &mut Context<'js, E>) -> Option<String> {
        self.0.get_typed::<String>(cx, "name").ok()
    }

    pub fn message(&self, cx: &mut Context<'js, E>) -> Option<String> {
        self.0.get_typed::<String>(cx, "message").ok()
    }

    pub fn stack(&self, cx: &mut Context<'js, E>) -> Option<String> {
        self.0.get_typed::<String>(cx, "stack").ok()
    }
}

impl<'js, E: Engine> Clone for JsException<'js, E>
where
    Object<'js, E>: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js, E: Engine> fmt::Debug for JsException<'js, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsException").finish_non_exhaustive()
    }
}

impl<'js, E: Engine> fmt::Display for JsException<'js, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JS exception")
    }
}
