use crate::{Error, PropertyKey, Result};

pub trait Engine: Sized + 'static {
    type Runtime: crate::Runtime<Self>;
    type Context<'js>: 'js;
    type Value<'js>: 'js;
    type Object<'js>: 'js;
    type Function<'js>: 'js;
    type String<'js>: 'js;
    type Symbol<'js>: 'js;
    type Key<'js>: 'js;
    type PreparedKeyData: 'static;
    type RawArgs<'js>: 'js;
    type PersistentValue: 'static;
    const ENGINE_NAME: &'static str;

    fn enter<'js>(runtime: &'js mut Self::Runtime) -> Self::Context<'js>;

    fn raw_args_len<'js>(args: &Self::RawArgs<'js>) -> usize;

    fn raw_args_get<'js>(args: &Self::RawArgs<'js>, index: usize) -> Option<Self::Value<'js>>;

    fn eval<'js>(
        cx: &mut Self::Context<'js>,
        src: &str,
        filename: Option<&str>,
    ) -> Result<Self::Value<'js>>;

    fn global_object<'js>(cx: &mut Self::Context<'js>) -> Self::Object<'js>;

    fn object_new<'js>(cx: &mut Self::Context<'js>) -> Result<Self::Object<'js>>;

    fn object_get<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<Self::Value<'js>>;

    fn object_set<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
        val: Self::Value<'js>,
    ) -> Result<()>;

    fn object_has<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool>;

    fn object_delete<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool>;

    fn function_call<'js>(
        cx: &mut Self::Context<'js>,
        func: &Self::Function<'js>,
        this: Self::Value<'js>,
        args: &[Self::Value<'js>],
    ) -> Result<Self::Value<'js>>;

    fn value_is_undefined<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_null<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_boolean<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_number<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_string<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_object<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_function<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_array<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_symbol<'js>(val: &Self::Value<'js>) -> bool;
    fn value_is_bigint<'js>(val: &Self::Value<'js>) -> bool;

    fn make_undefined<'js>(cx: &mut Self::Context<'js>) -> Self::Value<'js>;
    fn make_null<'js>(cx: &mut Self::Context<'js>) -> Self::Value<'js>;
    fn make_bool<'js>(cx: &mut Self::Context<'js>, v: bool) -> Self::Value<'js>;
    fn make_i32<'js>(cx: &mut Self::Context<'js>, v: i32) -> Self::Value<'js>;
    fn make_f64<'js>(cx: &mut Self::Context<'js>, v: f64) -> Self::Value<'js>;

    fn make_string<'js>(cx: &mut Self::Context<'js>, s: &str) -> Result<Self::Value<'js>>;

    fn make_function<'js, F>(
        cx: &mut Self::Context<'js>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'js>>
    where
        F: crate::args::RawHostFn<Self> + 'static;

    fn make_constructor<'js, F>(
        cx: &mut Self::Context<'js>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'js>>
    where
        F: crate::args::RawHostFn<Self> + 'static,
    {
        Self::make_function(cx, name, func)
    }

    fn value_as_bool<'js>(val: &Self::Value<'js>) -> Option<bool>;

    fn value_as_f64<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Option<f64> {
        Self::value_is_number(val)
            .then(|| Self::value_to_f64(cx, val).ok())
            .flatten()
    }

    fn value_as_string<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Option<String> {
        Self::value_is_string(val)
            .then(|| Self::value_to_string(cx, val).ok())
            .flatten()
    }

    fn value_to_bool<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> bool;

    fn value_to_f64<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<f64>;

    fn value_to_string<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<String>;

    fn object_to_value<'js>(obj: Self::Object<'js>) -> Self::Value<'js>;

    fn value_as_object<'js>(val: Self::Value<'js>) -> Option<Self::Object<'js>>;

    fn function_to_value<'js>(f: Self::Function<'js>) -> Self::Value<'js>;

    fn value_as_function<'js>(val: Self::Value<'js>) -> Option<Self::Function<'js>>;

    fn function_to_object<'js>(f: Self::Function<'js>) -> Self::Object<'js>;

    fn persist_value<'js>(
        cx: &mut Self::Context<'js>,
        val: Self::Value<'js>,
    ) -> Self::PersistentValue;

    fn restore_value<'js>(
        cx: &mut Self::Context<'js>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'js>>;

    fn catch_exception<'js>(cx: &mut Self::Context<'js>) -> Option<Self::Value<'js>> {
        let _ = cx;
        None
    }

    fn throw<'js>(cx: &mut Self::Context<'js>, value: Self::Value<'js>) -> Error;
}
