use crate::runtime::Runtime;

pub trait PersistentLike<R: Runtime>: Clone + 'static {
    fn new<'s, 'p: 's>(scope: &mut R::Scope<'s, 'p>, value: R::Value<'s>) -> Self;
    fn get<'s, 'p: 's>(&self, scope: &mut R::Scope<'s, 'p>) -> R::Value<'s>;
}

pub struct Global<R: Runtime> {
    handle: R::Persistent,
}

impl<R: Runtime> Global<R> {
    pub fn new<'s, 'p: 's>(scope: &mut R::Scope<'s, 'p>, value: R::Value<'s>) -> Self {
        Self {
            handle: R::Persistent::new(scope, value),
        }
    }

    pub fn get<'s, 'p: 's>(&self, scope: &mut R::Scope<'s, 'p>) -> R::Value<'s> {
        self.handle.get(scope)
    }

    pub fn handle(&self) -> &R::Persistent {
        &self.handle
    }
}

impl<R: Runtime> Clone for Global<R> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}
