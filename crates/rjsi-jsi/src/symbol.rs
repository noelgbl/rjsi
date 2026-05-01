use crate::Engine;

pub struct Symbol<'cx, E: Engine> {
    pub(crate) raw: E::Symbol<'cx>,
}

impl<'cx, E: Engine> Symbol<'cx, E> {
    pub fn new(raw: E::Symbol<'cx>) -> Self {
        Self { raw }
    }

    pub fn as_raw(&self) -> &E::Symbol<'cx> {
        &self.raw
    }
}
