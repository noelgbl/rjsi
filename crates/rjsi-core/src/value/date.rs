use crate::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct JsDate<'js, E: JsEngine + 'static> {
    inner: JsValue<'js, E>,
}

impl<'js, E: JsEngine> JsDate<'js, E> {
    /// Create a new JsDate from epoch milliseconds
    pub fn new(ctx: JsContext<'js, E>, epoch_ms: f64) -> Self {
        let value = E::Value::create_date(ctx.native_context(), epoch_ms);
        Self {
            inner: JsValue::from_raw(ctx, value),
        }
    }

    /// Create a JsDate for the current time
    pub fn now(ctx: JsContext<'js, E>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64;
        Self::new(ctx, now)
    }

    /// Create a JsDate from SystemTime
    pub fn from_system_time(ctx: JsContext<'js, E>, time: SystemTime) -> Self {
        let epoch_ms = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64;
        Self::new(ctx, epoch_ms)
    }

    /// Get the epoch milliseconds by calling JavaScript getTime() method
    pub fn get_time(&self) -> JsResult<f64>
    where
        E::Value: JsValueImpl + JsTypeOf + JsValueConversion + JsObjectOps,
    {
        let date_obj = self.inner.clone().into_object().ok_or_else(|| {
            HostError::new(crate::error::E_TYPE, "Date is not an object").with_name("TypeError")
        })?;

        let get_time = date_obj.get::<_, JsFunc<'js, E>>("getTime")?;

        get_time.call::<_, f64>(Some(date_obj), ())
    }

    /// Convert to SystemTime
    pub fn to_system_time(&self) -> JsResult<SystemTime>
    where
        E::Value: JsValueImpl + JsTypeOf + JsValueConversion + JsObjectOps,
    {
        let epoch_ms = self.get_time()?;
        let duration = std::time::Duration::from_millis(epoch_ms as u64);
        Ok(UNIX_EPOCH + duration)
    }

    /// Get the underlying JsValue
    pub fn as_js_value(&self) -> &JsValue<'js, E> {
        &self.inner
    }

    /// Convert into the underlying JsValue
    pub fn into_js_value(self) -> JsValue<'js, E> {
        self.inner
    }

    /// Convert into the underlying engine value
    pub fn into_value(self) -> E::Value {
        self.inner.into_inner()
    }

    /// Borrow the underlying engine value
    pub fn as_value(&self) -> &E::Value {
        self.inner.as_value()
    }
}

impl<'js, E: JsEngine> From<JsDate<'js, E>> for JsValue<'js, E> {
    fn from(date: JsDate<'js, E>) -> Self {
        date.inner
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsDate<'js, E>
where
    E::Value: JsTypeOf,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if !value.is_date() {
            return Err(HostError::new(crate::error::E_TYPE, "Value is not a Date")
                .with_name("TypeError")
                .into());
        }
        Ok(JsDate { inner: value })
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsDate<'js, E> {
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self.inner
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for SystemTime
where
    E::Value: JsTypeOf + JsValueConversion + JsObjectOps,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        let js_date = JsDate::from_js_value(ctx, value)?;
        js_date.to_system_time()
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for SystemTime {
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let epoch_ms = self
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64;
        JsValue::from_raw(
            ctx.clone(),
            E::Value::create_date(ctx.native_context(), epoch_ms),
        )
    }
}
