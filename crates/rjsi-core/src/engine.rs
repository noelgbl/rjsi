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
    type PreparedKeyData: 'static;
    type RawArgs<'cx>: 'cx;

    fn enter<'rt>(runtime: &'rt mut Self::Runtime) -> Self::Context<'rt>;

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize;

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>>;

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        filename: Option<&str>,
    ) -> JsResult<Self::Value<'rt>>;

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt>;

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> JsResult<Self::Object<'rt>>;

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<Self::Value<'rt>>;

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> JsResult<()>;

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<bool>;

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<bool>;

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> JsResult<Self::Value<'rt>>;

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

    fn make_undefined<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt>;
    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt>;
    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt>;
    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt>;
    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt>;

    fn make_string<'rt>(
        cx: &mut Self::Context<'rt>,
        s: &str,
    ) -> JsResult<Self::Value<'rt>>;

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> JsResult<Self::Function<'rt>>
    where
        F: crate::args::RawHostFn<Self> + 'static;

    fn make_constructor<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> JsResult<Self::Function<'rt>>
    where
        F: crate::args::RawHostFn<Self> + 'static,
    {
        Self::make_function(cx, name, func)
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool>;

    fn value_to_f64<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<f64>;

    fn value_to_string_utf8<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<String>;

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx>;

    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>>;

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx>;

    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>>;

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx>;
}
