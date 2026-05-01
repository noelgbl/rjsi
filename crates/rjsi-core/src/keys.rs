use crate::{Engine, JsResult};

#[repr(transparent)]
pub struct Key<'cx, E: Engine> {
    pub(crate) raw: E::Key<'cx>,
}

impl<'cx, E: Engine> Key<'cx, E> {
    pub fn new(raw: E::Key<'cx>) -> Self {
        Self { raw }
    }

    pub fn into_raw(self) -> E::Key<'cx> {
        self.raw
    }
}

pub enum PropertyKey<'cx, E: Engine> {
    Str(&'cx str),
    Interned(E::Key<'cx>),
    Symbol(E::Symbol<'cx>),
    Index(u32),
}

pub trait IntoKey<'cx, E: Engine> {
    fn into_key(self) -> PropertyKey<'cx, E>;
}

impl<'cx, E: Engine> IntoKey<'cx, E> for PropertyKey<'cx, E> {
    fn into_key(self) -> PropertyKey<'cx, E> {
        self
    }
}

impl<'cx, E: Engine> IntoKey<'cx, E> for &'cx str {
    fn into_key(self) -> PropertyKey<'cx, E> {
        PropertyKey::Str(self)
    }
}

impl<'cx, E: Engine> IntoKey<'cx, E> for u32 {
    fn into_key(self) -> PropertyKey<'cx, E> {
        PropertyKey::Index(self)
    }
}

impl<'cx, E: Engine> IntoKey<'cx, E> for Key<'cx, E> {
    fn into_key(self) -> PropertyKey<'cx, E> {
        PropertyKey::Interned(self.into_raw())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct StaticKeySlot(pub u32);

pub trait KeyCache<E: Engine> {
    fn get_or_intern<'cx>(
        &mut self,
        cx: &mut crate::Context<'cx, E>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, E, Key<'cx, E>>;
}

pub trait InternKey<E: Engine> {
    fn intern_str<'cx>(
        &mut self,
        cx: &mut crate::Context<'cx, E>,
        s: &str,
    ) -> JsResult<'cx, E, Key<'cx, E>>;
}
