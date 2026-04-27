use crate::Runtime;
use super::{ClassDescriptor, NativeClass, NativeRef};

/// Implemented by engine contexts that support native classes.
/// Separated from [`crate::context::ContextLike`] so non-class backends remain valid.
pub trait ClassRegistry<R: Runtime> {
    /// Register a class (idempotent) and install its constructor on `globalThis` under
    /// [`NativeClass::NAME`] when applicable.
    ///
    /// Implementations allocate engine values using the provided `scope`.
    fn register_class<'s, T: NativeClass>(
        &self,
        scope: &mut R::Scope<'s, 's>,
        descriptor: &'static ClassDescriptor<R>,
    ) -> Result<(), R::Error>;

    /// Wrap a Rust value into a JS object without invoking the JS constructor.
    /// The class must have been registered first.
    fn wrap_native<'s, T: NativeClass>(
        scope: &mut R::Scope<'s, 's>,
        value: T,
    ) -> Result<R::Value<'s>, R::Error>;

    /// Extract a typed reference to the Rust value inside a JS object.
    /// Returns `None` if `value` is not an instance of `T`.
    fn unwrap_native<'s, T: NativeClass>(
        scope: &mut R::Scope<'s, 's>,
        value: R::Value<'s>,
    ) -> Option<NativeRef<'s, T>>;

    /// Test if a JS value is an instance of the registered class `T`.
    fn instance_of<'s, T: NativeClass>(
        scope: &mut R::Scope<'s, 's>,
        value: R::Value<'s>,
    ) -> bool {
        Self::unwrap_native::<T>(scope, value).is_some()
    }
}

/// Scope helpers for [`ClassRegistry`] ([`wrap_native`](ClassRegistry::wrap_native), etc.).
pub trait ClassScopeExt<'s, 'p, R: Runtime>: Sized {
    /// Wrap a Rust value as a JS object for this scope.
    fn wrap_native<T: NativeClass>(&mut self, value: T) -> Result<R::Value<'s>, R::Error>;

    /// Unwrap a JS object to a native reference.
    fn unwrap_native<T: NativeClass>(&mut self, value: R::Value<'s>) -> Option<NativeRef<'s, T>>;

    /// [`ClassRegistry::instance_of`] as a method.
    fn instance_of<T: NativeClass>(&mut self, value: R::Value<'s>) -> bool;
}

impl<'s, 'p, R> ClassScopeExt<'s, 'p, R> for R::Scope<'s, 'p>
where
    R: Runtime,
    R::Context: ClassRegistry<R>,
    's: 'p,
{
    fn wrap_native<T: NativeClass>(&mut self, value: T) -> Result<R::Value<'s>, R::Error> {
        <R::Context as ClassRegistry<R>>::wrap_native::<T>(self, value)
    }

    fn unwrap_native<T: NativeClass>(&mut self, value: R::Value<'s>) -> Option<NativeRef<'s, T>> {
        <R::Context as ClassRegistry<R>>::unwrap_native::<T>(self, value)
    }

    fn instance_of<T: NativeClass>(&mut self, value: R::Value<'s>) -> bool {
        <R::Context as ClassRegistry<R>>::instance_of::<T>(self, value)
    }
}
