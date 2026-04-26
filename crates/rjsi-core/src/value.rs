use crate::runtime::Runtime;

pub trait ValueLike<'s, R: Runtime>: Clone + Sized {
    fn is_undefined(&self) -> bool;
    fn is_null(&self) -> bool;
    fn is_boolean(&self) -> bool;
    fn is_number(&self) -> bool;
    fn is_string(&self) -> bool;
    fn is_object(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_function(&self) -> bool;
    fn is_array_buffer(&self) -> bool;

    fn as_bool(&self, scope: &mut R::Scope<'s, '_>) -> Option<bool>;
    fn as_i32(&self, scope: &mut R::Scope<'s, '_>) -> Option<i32>;
    fn as_f64(&self, scope: &mut R::Scope<'s, '_>) -> Option<f64>;

    fn with_str<F, T>(&self, scope: &mut R::Scope<'s, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&str) -> T;

    fn to_string_lossy(&self, scope: &mut R::Scope<'s, '_>) -> Option<String>;

    fn get(&self, scope: &mut R::Scope<'s, '_>, key: &str) -> R::Value<'s>;
    fn set(&self, scope: &mut R::Scope<'s, '_>, key: &str, val: R::Value<'s>);
    fn has(&self, scope: &mut R::Scope<'s, '_>, key: &str) -> bool;
    fn delete(&self, scope: &mut R::Scope<'s, '_>, key: &str) -> bool;

    fn get_index(&self, scope: &mut R::Scope<'s, '_>, i: u32) -> R::Value<'s>;
    fn set_index(&self, scope: &mut R::Scope<'s, '_>, i: u32, val: R::Value<'s>);
    fn length(&self, scope: &mut R::Scope<'s, '_>) -> u32;

    fn with_bytes<F, T>(&self, scope: &mut R::Scope<'s, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&[u8]) -> T;

    fn call(
        &self,
        scope: &mut R::Scope<'s, '_>,
        this: R::Value<'s>,
        args: &[R::Value<'s>],
    ) -> Result<R::Value<'s>, R::Error>;
}

pub trait JsFunction<'s, R: Runtime>: Clone + Sized {}
