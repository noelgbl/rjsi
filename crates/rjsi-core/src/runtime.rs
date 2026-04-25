use crate::{HostFunction, JsGlobalHandle, JsResult, JsValueType, PropertyAttributes, Source};

/// Engine descriptor for a hot-path, scope-bound JavaScript binding.
///
/// `Value<'js>` is intentionally scoped. It is not required to be `Clone`,
/// `Hash`, or `'static`; values that must outlive a scope must be explicitly
/// rooted through [`crate::Global`].
pub trait JsEngine: Sized + 'static {
    type Scope<'js>: JsScope<'js, Engine = Self>;
    type Value<'js>: 'js;
    type PropertyKey<'js>: 'js;
    type Global: JsGlobalHandle<Self>;
    type HostArgs<'a, 'js>: crate::HostArgs<'a, 'js, Self>
    where
        'js: 'a;

    fn name() -> &'static str;
    fn version() -> String;
}

/// Active engine scope. All hot-path operations happen through this trait so
/// backends can reuse a single native scope/context for an entire closure.
pub trait JsScope<'js> {
    type Engine: JsEngine;

    fn eval(
        &mut self,
        source: Source,
    ) -> JsResult<<Self::Engine as JsEngine>::Value<'js>>;

    fn global(&mut self) -> <Self::Engine as JsEngine>::Value<'js>;
    fn undefined(&mut self) -> <Self::Engine as JsEngine>::Value<'js>;
    fn null(&mut self) -> <Self::Engine as JsEngine>::Value<'js>;
    fn boolean(&mut self, value: bool) -> <Self::Engine as JsEngine>::Value<'js>;
    fn number(&mut self, value: f64) -> <Self::Engine as JsEngine>::Value<'js>;
    fn string(&mut self, value: &str) -> <Self::Engine as JsEngine>::Value<'js>;
    fn object(&mut self) -> <Self::Engine as JsEngine>::Value<'js>;
    fn array(&mut self, len: u32) -> <Self::Engine as JsEngine>::Value<'js>;
    fn array_buffer_copy(&mut self, bytes: &[u8]) -> <Self::Engine as JsEngine>::Value<'js>;
    fn host_function<F>(
        &mut self,
        name: &'static str,
        function: F,
    ) -> Result<<Self::Engine as JsEngine>::Value<'js>, <Self::Engine as JsEngine>::Value<'js>>
    where
        F: HostFunction<Self::Engine>;

    fn value_type(&mut self, value: &<Self::Engine as JsEngine>::Value<'js>) -> JsValueType;
    fn to_boolean(&mut self, value: &<Self::Engine as JsEngine>::Value<'js>) -> Option<bool>;
    fn to_number(&mut self, value: &<Self::Engine as JsEngine>::Value<'js>) -> Option<f64>;
    fn to_string(&mut self, value: &<Self::Engine as JsEngine>::Value<'js>) -> Option<String>;

    fn property_key(&mut self, key: &str) -> <Self::Engine as JsEngine>::PropertyKey<'js>;
    fn static_property_key(
        &mut self,
        key: &'static str,
    ) -> <Self::Engine as JsEngine>::PropertyKey<'js> {
        self.property_key(key)
    }

    fn get_property(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &<Self::Engine as JsEngine>::PropertyKey<'js>,
    ) -> Result<Option<<Self::Engine as JsEngine>::Value<'js>>, <Self::Engine as JsEngine>::Value<'js>>;

    fn set_property(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &<Self::Engine as JsEngine>::PropertyKey<'js>,
        value: &<Self::Engine as JsEngine>::Value<'js>,
    ) -> Result<(), <Self::Engine as JsEngine>::Value<'js>>;

    fn has_property(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &<Self::Engine as JsEngine>::PropertyKey<'js>,
    ) -> Result<bool, <Self::Engine as JsEngine>::Value<'js>>;

    fn delete_property(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &<Self::Engine as JsEngine>::PropertyKey<'js>,
    ) -> Result<bool, <Self::Engine as JsEngine>::Value<'js>>;

    fn define_property(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        key: &<Self::Engine as JsEngine>::PropertyKey<'js>,
        value: &<Self::Engine as JsEngine>::Value<'js>,
        attributes: PropertyAttributes,
    ) -> Result<(), <Self::Engine as JsEngine>::Value<'js>>;

    fn get_index(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        index: u32,
    ) -> Result<Option<<Self::Engine as JsEngine>::Value<'js>>, <Self::Engine as JsEngine>::Value<'js>>;

    fn set_index(
        &mut self,
        object: &<Self::Engine as JsEngine>::Value<'js>,
        index: u32,
        value: &<Self::Engine as JsEngine>::Value<'js>,
    ) -> Result<(), <Self::Engine as JsEngine>::Value<'js>>;

    fn call_function(
        &mut self,
        function: &<Self::Engine as JsEngine>::Value<'js>,
        this: Option<&<Self::Engine as JsEngine>::Value<'js>>,
        argv: &[<Self::Engine as JsEngine>::Value<'js>],
    ) -> Result<<Self::Engine as JsEngine>::Value<'js>, <Self::Engine as JsEngine>::Value<'js>>;

    fn throw(
        &mut self,
        value: <Self::Engine as JsEngine>::Value<'js>,
    ) -> <Self::Engine as JsEngine>::Value<'js>;
}

/// Runtime owner. Entering a runtime produces an active scope; values cannot
/// escape the closure unless explicitly rooted.
pub trait JsRuntime: 'static {
    type Engine: JsEngine;

    fn with_scope<R>(
        &self,
        f: impl for<'js> FnOnce(&mut <Self::Engine as JsEngine>::Scope<'js>) -> JsResult<R>,
    ) -> JsResult<R>;
}
