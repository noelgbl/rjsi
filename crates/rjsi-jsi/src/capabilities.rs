use crate::{runtime::Runtime, scope::ScopeLike};

pub trait ScopeArrayBuffer<'s, R: Runtime> {
    fn array_buffer_zero_copy(&mut self, bytes: &[u8]) -> R::Value<'s>;
    fn array_buffer_zero_copy_supported(&self) -> bool;
}

impl<'s, 'p, R> ScopeArrayBuffer<'s, R> for R::Scope<'s, 'p>
where
    R: Runtime,
{
    fn array_buffer_zero_copy(&mut self, bytes: &[u8]) -> R::Value<'s> {
        <R::Scope<'s, 'p> as ScopeLike<'s, 'p, R>>::array_buffer_zero_copy(self, bytes)
    }

    fn array_buffer_zero_copy_supported(&self) -> bool {
        <R::Scope<'s, 'p> as ScopeLike<'s, 'p, R>>::array_buffer_zero_copy_supported(self)
    }
}
