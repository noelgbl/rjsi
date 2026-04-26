use crate::{callback::Args, runtime::Runtime};

pub enum TryCatchResult<V, E> {
    Ok(V),
    Exception(E),
}

impl<V, E> TryCatchResult<V, E> {
    pub fn into_result(self) -> Result<V, E> {
        match self {
            TryCatchResult::Ok(v) => Ok(v),
            TryCatchResult::Exception(e) => Err(e),
        }
    }
}

pub trait ScopeLike<'s, 'p: 's, R: Runtime> {
    fn with_scope<'s2, F, T>(&'s2 mut self, f: F) -> T
    where
        's: 's2,
        F: FnOnce(&mut R::Scope<'s2, 's>) -> T;

    fn eval(&mut self, src: &str) -> Result<R::Value<'s>, R::Error>;

    fn global(&mut self) -> R::Value<'s>;
    fn undefined(&mut self) -> R::Value<'s>;
    fn null(&mut self) -> R::Value<'s>;
    fn boolean(&mut self, value: bool) -> R::Value<'s>;
    fn integer(&mut self, value: i32) -> R::Value<'s>;
    fn number(&mut self, value: f64) -> R::Value<'s>;
    fn string(&mut self, value: &str) -> R::Value<'s>;
    fn object(&mut self) -> R::Value<'s>;
    fn array(&mut self, len: u32) -> R::Value<'s>;
    fn array_buffer_copy(&mut self, bytes: &[u8]) -> R::Value<'s>;

    fn try_catch<F>(&mut self, f: F) -> TryCatchResult<R::Value<'s>, R::Error>
    where
        F: FnOnce(&mut R::Scope<'_, '_>) -> Result<R::Value<'s>, R::Error>;

    fn array_buffer_zero_copy(&mut self, data: &'s [u8]) -> R::Value<'s>;

    fn function<F>(&mut self, f: F) -> Result<R::Value<'s>, R::Error>
    where
        F: for<'a> Fn(&mut R::Scope<'a, 'a>, Args<'a, R>) -> Result<R::Value<'a>, R::Error>
            + Send
            + Sync
            + 'static;
}
