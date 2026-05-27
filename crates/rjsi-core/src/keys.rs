use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::Engine;

static NEXT_PREPARED_KEY_ID: AtomicU64 = AtomicU64::new(0);

struct PreparedKeyInner {
    id: u64,
    name: Box<str>,
}

pub struct PreparedKey<E: Engine> {
    inner: Arc<PreparedKeyInner>,
    _marker: PhantomData<fn() -> E>,
}

impl<E: Engine> PreparedKey<E> {
    pub fn new(name: impl Into<String>) -> Self {
        let id = NEXT_PREPARED_KEY_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: Arc::new(PreparedKeyInner {
                id,
                name: name.into().into_boxed_str(),
            }),
            _marker: PhantomData,
        }
    }

    pub fn id(&self) -> u64 {
        self.inner.id
    }

    pub fn as_str(&self) -> &str {
        &self.inner.name
    }
}

impl<E: Engine> Clone for PreparedKey<E> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _marker: PhantomData,
        }
    }
}

impl<E: Engine> std::fmt::Debug for PreparedKey<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedKey")
            .field("id", &self.id())
            .field("name", &self.as_str())
            .finish()
    }
}

impl<E: Engine> PartialEq for PreparedKey<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl<E: Engine> Eq for PreparedKey<E> {}

impl<E: Engine> std::hash::Hash for PreparedKey<E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

/// A key used to address an object property.
///
/// Carrying a [`crate::Symbol`] wrapper rather than the raw `E::Symbol<'js>`
/// keeps the enum invariant in `'js` — see [`crate::markers::Invariant`].
pub enum PropertyKey<'js, E: Engine> {
    Str(&'js str),
    Prepared(PreparedKey<E>),
    Symbol(crate::Symbol<'js, E>),
    Index(u32),
}

pub trait IntoKey<'js, E: Engine> {
    fn into_key(self) -> PropertyKey<'js, E>;
}

impl<'js, E: Engine> IntoKey<'js, E> for PropertyKey<'js, E> {
    fn into_key(self) -> PropertyKey<'js, E> {
        self
    }
}

impl<'js, E: Engine> IntoKey<'js, E> for &'js str {
    fn into_key(self) -> PropertyKey<'js, E> {
        PropertyKey::Str(self)
    }
}

impl<'js, E: Engine> IntoKey<'js, E> for u32 {
    fn into_key(self) -> PropertyKey<'js, E> {
        PropertyKey::Index(self)
    }
}

impl<'js, E: Engine> IntoKey<'js, E> for PreparedKey<E> {
    fn into_key(self) -> PropertyKey<'js, E> {
        PropertyKey::Prepared(self)
    }
}

impl<'js, E: Engine> IntoKey<'js, E> for &PreparedKey<E> {
    fn into_key(self) -> PropertyKey<'js, E> {
        PropertyKey::Prepared(self.clone())
    }
}
