use crate::runtime::Runtime;

pub trait ScopeArrayBuffer<'s, R: Runtime> {
    fn array_buffer_zero_copy(&mut self, bytes: &[u8]) -> R::Value<'s>;
}
