use crate::{
    FromJsValue, HostError, JsContext, JsEngine, JsFunc, JsResult, JsTypeOf, JsValue,
    JsValueConversion, JsValueImpl, JsonToJsValue, RjsiJSError,
};
use std::fmt;
use std::ops::Deref;

mod property;
pub use property::{PropertyAttributes, PropertyDescriptor, PropertyKey};

use super::IntoJsValue;

#[derive(Hash, PartialEq)]
pub struct JsObject<'js, E: JsEngine + 'static>(JsValue<'js, E>);

impl<'js, E: JsEngine> Clone for JsObject<'js, E>
where
    E::Value: Clone,
{
    fn clone(&self) -> Self {
        JsObject(self.0.clone())
    }
}

impl<'js, E: JsEngine> From<JsValue<'js, E>> for JsObject<'js, E>
where
    E::Value: JsValueImpl,
{
    fn from(v: JsValue<'js, E>) -> Self {
        JsObject(v)
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsObject<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_object() {
            Ok(value.into())
        } else {
            Err(HostError::not_object().into())
        }
    }
}

impl<'js, E: JsEngine> Deref for JsObject<'js, E> {
    type Target = JsValue<'js, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsObject<'js, E> {
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.0
    }
}

pub trait JsObjectOps: JsValueConversion + JsTypeOf {
    fn new_object(ctx: &Self::Context) -> Self;

    fn make_instance(ctx: &Self::Context, constructor: Self, data: *mut ()) -> Self;

    fn instance_of(&self, constructor: Self) -> bool;

    fn get_opaque(&self) -> *mut ();

    fn del_property(&self, key: Self) -> Result<bool, Self>;

    fn has_property(&self, key: Self) -> Result<bool, Self>;

    fn set_property(&self, key: Self, value: Self) -> Result<(), Self>;

    fn set_prototype(&self, prototype: Self) -> bool;

    fn define_property(
        &self,
        key: Self,
        value: Self,
        getter: Self,
        setter: Self,
        attributes: PropertyAttributes,
    ) -> Result<(), Self>;

    fn get_property(&self, key: Self) -> Result<Option<Self>, Self>;

    fn get_own_property_names(&self) -> Result<Vec<Self>, Self>;
}

impl<'js, E: JsEngine> JsObject<'js, E>
where
    E::Value: JsObjectOps,
{
    pub fn context(&self) -> JsContext<'js, E> {
        self.0.context()
    }

    pub fn new(ctx: JsContext<'js, E>) -> Self {
        let value = E::Value::new_object(ctx.native_context());
        let v = JsValue::from_raw(ctx.clone(), value);
        JsObject::from_js_value(ctx, v).unwrap()
    }

    pub fn from_raw(ctx: JsContext<'js, E>, value: E::Value) -> Self {
        JsValue::from_raw(ctx, value).into()
    }

    pub fn from_json_string(ctx: JsContext<'js, E>, json: &str) -> JsResult<Self> {
        let v = json.json_to_js_value(ctx)?;
        Ok(JsObject(v))
    }

    fn thrown_error(ctx: JsContext<'js, E>, thrown: E::Value) -> RjsiJSError {
        let v = JsValue::from_raw(ctx.clone(), thrown);
        RjsiJSError::from_thrown_value(ctx, v)
    }

    pub fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        <JsObject<'js, E> as FromJsValue<'js, E>>::from_js_value(ctx, value)
    }

    pub fn to_json_string(&self) -> JsResult<String> {
        let ctx = self.context();
        let json = ctx.global()?.get::<_, JsObject<'js, E>>("JSON")?;
        let stringify = json.get::<_, JsFunc<'js, E>>("stringify")?;
        stringify.call::<_, String>(None, (self.clone(),))
    }

    pub fn set<'a, K, KV>(&'a self, k: K, kv: KV) -> JsResult<&'a Self>
    where
        K: Into<PropertyKey<'a, E>>,
        KV: IntoJsValue<'js, E>,
    {
        let ctx = self.context();
        let key = k.into().into_value(ctx.clone());
        let jv = kv.into_js_value(ctx.clone());
        self.as_value()
            .set_property(key, jv.into_inner())
            .map_err(|thrown| Self::thrown_error(ctx, thrown))?;
        Ok(self)
    }

    pub fn define_property<'a, K>(
        &'a self,
        k: K,
        descriptor: PropertyDescriptor<'js, E>,
    ) -> JsResult<&'a Self>
    where
        K: Into<PropertyKey<'a, E>>,
    {
        descriptor.define_on(self, k)?;
        Ok(self)
    }

    pub fn delete<'a, K>(&'a self, k: K) -> JsResult<bool>
    where
        K: Into<PropertyKey<'a, E>>,
    {
        let ctx = self.context();
        let key = k.into().into_value(ctx.clone());
        self.as_value()
            .del_property(key)
            .map_err(|thrown| Self::thrown_error(ctx, thrown))
    }

    pub fn has_property<'a, K>(&self, k: K) -> JsResult<bool>
    where
        K: Into<PropertyKey<'a, E>>,
    {
        let ctx = self.context();
        let key = k.into().into_value(ctx.clone());
        self.as_value()
            .has_property(key)
            .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))
    }

    pub fn get<'a, K, T>(&'a self, k: K) -> JsResult<T>
    where
        K: Into<PropertyKey<'a, E>>,
        T: FromJsValue<'js, E>,
    {
        let ctx = self.context();
        let key = k.into();
        let kv = key.clone().into_value(ctx.clone());
        let value = self
            .as_value()
            .get_property(kv.clone())
            .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
            .map(|value| JsValue::from_raw(ctx.clone(), value));

        let value = match value {
            Some(value) if !value.is_undefined() => value,
            Some(value) => {
                if !self
                    .as_value()
                    .has_property(kv)
                    .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
                {
                    return Err(HostError::property_not_found(key).into());
                }
                value
            }
            None => {
                if !self
                    .as_value()
                    .has_property(kv)
                    .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
                {
                    return Err(HostError::property_not_found(key).into());
                }
                JsValue::undefined(ctx.clone())
            }
        };

        T::from_js_value(ctx, value)
    }

    pub fn get_opt<'a, K, T>(&'a self, k: K) -> JsResult<Option<T>>
    where
        K: Into<PropertyKey<'a, E>>,
        T: FromJsValue<'js, E>,
    {
        let ctx = self.context();
        let key = k.into().into_value(ctx.clone());
        let value = self
            .as_value()
            .get_property(key.clone())
            .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
            .map(|value| JsValue::from_raw(ctx.clone(), value));

        let value = match value {
            Some(value) if !value.is_undefined() => value,
            Some(value) => {
                if !self
                    .as_value()
                    .has_property(key)
                    .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
                {
                    return Ok(None);
                }
                value
            }
            None => {
                if !self
                    .as_value()
                    .has_property(key)
                    .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
                {
                    return Ok(None);
                }
                JsValue::undefined(ctx.clone())
            }
        };

        T::from_js_value(ctx, value).map(Some)
    }
}

impl<'js, E: JsEngine> JsObject<'js, E> {
    pub fn into_value(self) -> E::Value {
        self.0.into_inner()
    }

    pub fn as_js_value(&self) -> &JsValue<'js, E> {
        &self.0
    }

    pub fn as_mut_value(&mut self) -> &mut E::Value {
        &mut self.0.inner
    }
}

pub struct Entry<'js, E: JsEngine + 'static> {
    pub key: JsValue<'js, E>,
    pub value: JsValue<'js, E>,
}

impl<'js, E: JsEngine> Entry<'js, E> {
    pub fn key(&self) -> &JsValue<'js, E> {
        &self.key
    }

    pub fn value(&self) -> &JsValue<'js, E> {
        &self.value
    }

    pub fn into_tuple(self) -> (JsValue<'js, E>, JsValue<'js, E>) {
        (self.key, self.value)
    }

    pub fn try_into<K, T>(self) -> JsResult<(K, T)>
    where
        K: FromJsValue<'js, E>,
        T: FromJsValue<'js, E>,
    {
        let ctx = self.key.context();
        let k = K::from_js_value(ctx.clone(), self.key)?;
        let t = T::from_js_value(ctx, self.value)?;
        Ok((k, t))
    }
}

impl<'js, E: JsEngine> JsObject<'js, E>
where
    E::Value: JsObjectOps,
{
    pub fn entries(&self) -> JsResult<Vec<Entry<'js, E>>> {
        let ctx = self.context();
        let mut entries = Vec::new();
        let keys = self.own_keys()?;

        for key in keys {
            if let Some(value) = self
                .as_value()
                .get_property(key.clone())
                .map_err(|thrown| Self::thrown_error(ctx.clone(), thrown))?
            {
                entries.push(Entry {
                    key: JsValue::from_raw(ctx.clone(), key),
                    value: JsValue::from_raw(ctx.clone(), value),
                });
            }
        }

        Ok(entries)
    }

    pub fn entries_as<K, V2>(&self) -> JsResult<Vec<(K, V2)>>
    where
        K: FromJsValue<'js, E>,
        V2: FromJsValue<'js, E>,
    {
        self.entries()?
            .into_iter()
            .map(|entry| entry.try_into::<K, V2>())
            .collect()
    }

    pub fn own_keys(&self) -> JsResult<Vec<E::Value>> {
        let ctx = self.context();
        self.as_value()
            .get_own_property_names()
            .map_err(|thrown| Self::thrown_error(ctx, thrown))
    }

    pub fn values(&self) -> JsResult<impl Iterator<Item = JsValue<'js, E>> + '_> {
        Ok(self.entries()?.into_iter().map(|entry| entry.value))
    }

    pub fn values_as<T>(&self) -> JsResult<Vec<T>>
    where
        T: FromJsValue<'js, E>,
    {
        let ctx = self.context();
        self.values()?
            .map(|v| T::from_js_value(ctx.clone(), v))
            .collect()
    }

    pub fn keys(&self) -> JsResult<impl Iterator<Item = JsValue<'js, E>> + '_> {
        Ok(self.entries()?.into_iter().map(|entry| entry.key))
    }

    pub fn keys_as<K>(&self) -> JsResult<Vec<K>>
    where
        K: FromJsValue<'js, E>,
    {
        let ctx = self.context();
        self.keys()?
            .map(|k| K::from_js_value(ctx.clone(), k))
            .collect()
    }
}

impl<'js, E: JsEngine> crate::function::JsParameterType for JsObject<'js, E> {}

impl<'js, E: JsEngine> fmt::Display for JsObject<'js, E>
where
    E::Value: JsTypeOf + JsValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<'js, E: JsEngine> fmt::Debug for JsObject<'js, E>
where
    E::Value: JsTypeOf + JsValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JsObject({})", self)
    }
}
