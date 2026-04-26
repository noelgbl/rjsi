use crate::runtime::Runtime;

pub trait ContextLike<R: Runtime>: 'static {
    fn with_scope<T>(
        &self,
        f: impl for<'s> FnOnce(&mut R::Scope<'s, 's>) -> Result<T, R::Error>,
    ) -> Result<T, R::Error>;
}
