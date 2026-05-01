use crate::Engine;

#[repr(transparent)]
pub struct JsString<'cx, E: Engine> {
    pub(crate) raw: E::String<'cx>,
}

impl<'cx, E: Engine> JsString<'cx, E> {
    pub fn new(raw: E::String<'cx>) -> Self {
        Self { raw }
    }

    pub fn as_raw(&self) -> &E::String<'cx> {
        &self.raw
    }
}
