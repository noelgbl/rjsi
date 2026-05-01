use crate::{JsResult, PropertyKey};

pub trait Engine: Sized + 'static {
    type Runtime;
    type Context<'rt>: 'rt;
    type Scope<'cx>: 'cx;
    type Value<'cx>: 'cx;
    type Object<'cx>: 'cx;
    type Function<'cx>: 'cx;
    type String<'cx>: 'cx;
    type Symbol<'cx>: 'cx;
    type Key<'cx>: 'cx;
    type Error<'cx>: 'cx;
    type RawArgs<'cx>: 'cx;

    fn enter<'rt>(runtime: &'rt mut Self::Runtime) -> Self::Context<'rt>;

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize;

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>>;

    fn eval<'cx>(
        cx: &mut Self::Context<'_>,
        src: &str,
        filename: Option<&str>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>>;

    fn global_object<'cx>(cx: &mut Self::Context<'_>) -> Self::Object<'cx>;

    fn object_new<'cx>(cx: &mut Self::Context<'_>) -> JsResult<'cx, Self, Self::Object<'cx>>;

    fn object_get<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>>;

    fn object_set<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
        val: Self::Value<'cx>,
    ) -> JsResult<'cx, Self, ()>;

    fn object_has<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool>;

    fn object_delete<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool>;

    fn function_call<'cx>(
        cx: &mut Self::Context<'_>,
        func: &Self::Function<'cx>,
        this: Self::Value<'cx>,
        args: &[Self::Value<'cx>],
    ) -> JsResult<'cx, Self, Self::Value<'cx>>;

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_string<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_object<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_function<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool;
    fn value_is_bigint<'cx>(val: &Self::Value<'cx>) -> bool;

    fn make_undefined<'cx>(cx: &mut Self::Context<'_>) -> Self::Value<'cx>;
    fn make_null<'cx>(cx: &mut Self::Context<'_>) -> Self::Value<'cx>;
    fn make_bool<'cx>(cx: &mut Self::Context<'_>, v: bool) -> Self::Value<'cx>;
    fn make_i32<'cx>(cx: &mut Self::Context<'_>, v: i32) -> Self::Value<'cx>;
    fn make_f64<'cx>(cx: &mut Self::Context<'_>, v: f64) -> Self::Value<'cx>;

    fn make_string<'cx>(
        cx: &mut Self::Context<'_>,
        s: &str,
    ) -> JsResult<'cx, Self, Self::Value<'cx>>;

    fn make_function<'cx, F>(
        cx: &mut Self::Context<'_>,
        name: &str,
        func: F,
    ) -> JsResult<'cx, Self, Self::Function<'cx>>
    where
        F: crate::args::RawHostFn<Self> + 'static;

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool>;

    fn value_to_f64<'cx>(
        cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, f64>;

    fn value_to_string_utf8<'cx>(
        cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, String>;

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx>;

    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>>;

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx>;

    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>>;

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx>;
}
