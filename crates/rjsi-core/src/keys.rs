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

pub enum PropertyKey<'cx, E: Engine> {
    Str(&'cx str),
    Prepared(&'cx PreparedKey<E>),
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

impl<'cx, E: Engine> IntoKey<'cx, E> for &'cx PreparedKey<E> {
    fn into_key(self) -> PropertyKey<'cx, E> {
        PropertyKey::Prepared(self)
    }
}
