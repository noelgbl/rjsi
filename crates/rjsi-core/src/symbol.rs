use std::marker::PhantomData;

use crate::Engine;
use crate::markers::Invariant;

#[repr(transparent)]
pub struct Symbol<'js, E: Engine> {
    pub(crate) raw: E::Symbol<'js>,
    _inv: PhantomData<Invariant<'js>>,
}

impl<'js, E: Engine> Clone for Symbol<'js, E>
where
    E::Symbol<'js>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _inv: PhantomData,
        }
    }
}

impl<'js, E: Engine> Symbol<'js, E> {
    pub fn new(raw: E::Symbol<'js>) -> Self {
        Self {
            raw,
            _inv: PhantomData,
        }
    }

    pub fn as_raw(&self) -> &E::Symbol<'js> {
        &self.raw
    }

    pub fn into_raw(self) -> E::Symbol<'js> {
        self.raw
    }
}
