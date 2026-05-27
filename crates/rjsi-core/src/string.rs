use std::marker::PhantomData;

use crate::Engine;
use crate::markers::Invariant;

#[repr(transparent)]
pub struct JsString<'js, E: Engine> {
    pub(crate) raw: E::String<'js>,
    _inv: PhantomData<Invariant<'js>>,
}

impl<'js, E: Engine> Clone for JsString<'js, E>
where
    E::String<'js>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _inv: PhantomData,
        }
    }
}

impl<'js, E: Engine> JsString<'js, E> {
    pub fn new(raw: E::String<'js>) -> Self {
        Self {
            raw,
            _inv: PhantomData,
        }
    }

    pub fn as_raw(&self) -> &E::String<'js> {
        &self.raw
    }
}
