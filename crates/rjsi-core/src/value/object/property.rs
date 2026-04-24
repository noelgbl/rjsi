use crate::{
    JsContext, JsEngine, JsFunc, JsObject, JsObjectOps, JsResult, JsSymbol, JsValue,
    JsValueConversion, JsValueImpl, RjsiJSError,
};
use std::fmt;

pub enum PropertyKey<'a, E: JsEngine + 'static> {
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Str(&'a str),
    Symbol(JsSymbol<'a, E>),
}

impl<'a, E: JsEngine> Clone for PropertyKey<'a, E>
where
    E::Value: Clone,
{
    fn clone(&self) -> Self {
        match self {
            PropertyKey::Int32(v) => PropertyKey::Int32(*v),
            PropertyKey::Uint32(v) => PropertyKey::Uint32(*v),
            PropertyKey::Int64(v) => PropertyKey::Int64(*v),
            PropertyKey::Uint64(v) => PropertyKey::Uint64(*v),
            PropertyKey::Str(s) => PropertyKey::Str(s),
            PropertyKey::Symbol(s) => PropertyKey::Symbol(s.clone()),
        }
    }
}

impl<E: JsEngine> From<i32> for PropertyKey<'_, E> {
    fn from(value: i32) -> Self {
        PropertyKey::Int32(value)
    }
}

impl<E: JsEngine> From<u32> for PropertyKey<'_, E> {
    fn from(value: u32) -> Self {
        PropertyKey::Uint32(value)
    }
}

impl<E: JsEngine> From<i64> for PropertyKey<'_, E> {
    fn from(value: i64) -> Self {
        PropertyKey::Int64(value)
    }
}

impl<E: JsEngine> From<u64> for PropertyKey<'_, E> {
    fn from(value: u64) -> Self {
        PropertyKey::Uint64(value)
    }
}

impl<'a, E: JsEngine> From<JsSymbol<'a, E>> for PropertyKey<'a, E> {
    fn from(value: JsSymbol<'a, E>) -> Self {
        PropertyKey::Symbol(value)
    }
}

impl<'a, 'b: 'a, E: JsEngine> From<&'b str> for PropertyKey<'a, E> {
    fn from(value: &'b str) -> Self {
        PropertyKey::Str(value)
    }
}

impl<E: JsEngine> PropertyKey<'_, E> {
    pub(crate) fn into_value(self, ctx: JsContext<'_, E>) -> E::Value
    where
        E::Value: JsValueConversion,
    {
        let engine = ctx.native_context();
        match self {
            Self::Int32(i) => (engine, i).into(),
            Self::Uint32(i) => (engine, i).into(),
            Self::Int64(i) => (engine, i).into(),
            Self::Uint64(i) => (engine, i).into(),
            Self::Str(s) => (engine, s).into(),
            Self::Symbol(s) => JsSymbol::into_value(s),
        }
    }
}

impl<E: JsEngine> fmt::Display for PropertyKey<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyKey::Int32(i) => write!(f, "{}", i),
            PropertyKey::Uint32(i) => write!(f, "{}", i),
            PropertyKey::Int64(i) => write!(f, "{}", i),
            PropertyKey::Uint64(i) => write!(f, "{}", i),
            PropertyKey::Str(s) => write!(f, "{}", s),
            PropertyKey::Symbol(s) => write!(
                f,
                "Symbol({})",
                s.descripiton().unwrap_or_else(|_| "".to_string())
            ),
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct PropertyAttributes(u32);

impl PropertyAttributes {
    const WRITABLE: u32 = 1;
    const ENUMERABLE: u32 = 1 << 1;
    const CONFIGURABLE: u32 = 1 << 2;
    const HAS_VALUE: u32 = 1 << 3;
    const HAS_GET: u32 = 1 << 4;
    const HAS_SET: u32 = 1 << 5;
    const HAS_WRITABLE: u32 = 1 << 6;
    const HAS_ENUMERABLE: u32 = 1 << 7;
    const HAS_CONFIGURABLE: u32 = 1 << 8;

    pub fn is_writable(&self) -> bool {
        self.0 & Self::WRITABLE != 0
    }

    #[doc(hidden)]
    pub fn has_writable(&self) -> bool {
        self.0 & Self::HAS_WRITABLE != 0
    }

    pub fn is_enumerable(&self) -> bool {
        self.0 & Self::ENUMERABLE != 0
    }

    #[doc(hidden)]
    pub fn has_enumerable(&self) -> bool {
        self.0 & Self::HAS_ENUMERABLE != 0
    }

    pub fn is_configurable(&self) -> bool {
        self.0 & Self::CONFIGURABLE != 0
    }

    #[doc(hidden)]
    pub fn has_configurable(&self) -> bool {
        self.0 & Self::HAS_CONFIGURABLE != 0
    }

    pub fn has_value(&self) -> bool {
        self.0 & Self::HAS_VALUE != 0
    }

    pub fn has_get(&self) -> bool {
        self.0 & Self::HAS_GET != 0
    }

    pub fn has_set(&self) -> bool {
        self.0 & Self::HAS_SET != 0
    }
}

pub struct PropertyDescriptor<'js, E: JsEngine + 'static> {
    value: Option<E::Value>,
    getter: Option<JsFunc<'js, E>>,
    setter: Option<JsFunc<'js, E>>,
    attributes: PropertyAttributes,
}

impl<'js, E: JsEngine> Default for PropertyDescriptor<'js, E>
where
    E::Value: JsObjectOps,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'js, E: JsEngine> PropertyDescriptor<'js, E>
where
    E::Value: JsObjectOps,
{
    fn thrown_error(ctx: JsContext<'js, E>, thrown: E::Value) -> RjsiJSError {
        let v = JsValue::from_raw(ctx.clone(), thrown);
        RjsiJSError::from_thrown_value(ctx, v)
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            value: None,
            getter: None,
            setter: None,
            attributes: PropertyAttributes::default(),
        }
    }

    #[must_use]
    pub fn from_value(value: JsValue<'js, E>) -> Self {
        Self::new().value(value)
    }

    #[must_use]
    pub fn from_rust<T>(ctx: JsContext<'js, E>, value: T) -> Self
    where
        T: crate::IntoJsValue<'js, E>,
    {
        Self::from_value(JsValue::from_rust(ctx, value))
    }

    #[must_use]
    pub fn from_getter(getter: JsFunc<'js, E>) -> Self {
        Self::new().getter(getter)
    }

    #[must_use]
    pub fn from_setter(setter: JsFunc<'js, E>) -> Self {
        Self::new().setter(setter)
    }

    #[must_use]
    pub fn from_accessor(getter: JsFunc<'js, E>, setter: JsFunc<'js, E>) -> Self {
        Self::from_getter(getter).setter(setter)
    }

    #[must_use]
    pub fn value(mut self, value: JsValue<'js, E>) -> Self {
        self.value = Some(value.into_inner());
        self
    }

    #[must_use]
    pub fn getter(mut self, getter: JsFunc<'js, E>) -> Self {
        self.getter = Some(getter);
        self
    }

    #[must_use]
    pub fn setter(mut self, setter: JsFunc<'js, E>) -> Self {
        self.setter = Some(setter);
        self
    }

    #[must_use]
    pub fn writable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_WRITABLE;
        self.attributes.0 |= PropertyAttributes::WRITABLE;
        self
    }

    #[must_use]
    pub fn readonly(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_WRITABLE;
        self.attributes.0 &= !PropertyAttributes::WRITABLE;
        self
    }

    #[must_use]
    pub fn enumerable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_ENUMERABLE;
        self.attributes.0 |= PropertyAttributes::ENUMERABLE;
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_ENUMERABLE;
        self.attributes.0 &= !PropertyAttributes::ENUMERABLE;
        self
    }

    #[must_use]
    pub fn configurable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_CONFIGURABLE;
        self.attributes.0 |= PropertyAttributes::CONFIGURABLE;
        self
    }

    #[must_use]
    pub fn non_configurable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_CONFIGURABLE;
        self.attributes.0 &= !PropertyAttributes::CONFIGURABLE;
        self
    }

    pub fn define_on<'a, K>(mut self, obj: &JsObject<'js, E>, k: K) -> JsResult<()>
    where
        K: Into<PropertyKey<'a, E>>,
        E::Value: JsObjectOps,
    {
        let ctx = obj.context();
        let undefined = E::Value::create_undefined(ctx.native_context());

        let value = self
            .value
            .inspect(|_| self.attributes.0 |= PropertyAttributes::HAS_VALUE)
            .unwrap_or(undefined.clone());

        let getter = self
            .getter
            .map(|g| {
                self.attributes.0 |= PropertyAttributes::HAS_GET;
                g.into_value()
            })
            .unwrap_or(undefined.clone());

        let setter = self
            .setter
            .map(|s| {
                self.attributes.0 |= PropertyAttributes::HAS_SET;
                s.into_value()
            })
            .unwrap_or(undefined.clone());

        let key = k.into().into_value(ctx.clone());

        obj.as_value()
            .define_property(key, value, getter, setter, self.attributes)
            .map_err(|thrown| Self::thrown_error(ctx, thrown))
    }
}
