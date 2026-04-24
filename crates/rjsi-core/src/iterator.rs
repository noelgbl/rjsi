use crate::{IntoJsValue, JsContext, JsEngine, JsObject, JsObjectOps, JsResult, JsValue};
use std::sync::{Arc, Mutex};

/// Core JavaScript iterator that wraps Rust iterators (no `Rc`/`RefCell`; uses `Arc<Mutex<...>>`).
pub struct JsIterator<'js, E: JsEngine, T>
where
    E::Value: JsObjectOps + Send + 'static,
    T: IntoJsValue<'js, E> + Send + 'static,
{
    inner: Arc<Mutex<Box<dyn Iterator<Item = T> + Send + 'static>>>,
    ctx: JsContext<'js, E>,
}

impl<'js, E, T> JsIterator<'js, E, T>
where
    E: JsEngine + 'static,
    E::Value: JsObjectOps + Send + 'static,
    T: IntoJsValue<'js, E> + Send + 'static,
{
    pub fn new<I>(iterable: I, ctx: JsContext<'js, E>) -> Self
    where
        I: IntoIterator<Item = T> + Send + 'static,
        I::IntoIter: Send + 'static,
    {
        Self {
            inner: Arc::new(Mutex::new(Box::new(iterable.into_iter()))),
            ctx,
        }
    }

    pub fn next(&self) -> JsResult<JsObject<'js, E>> {
        let result = JsObject::new(self.ctx.clone());
        let mut iter = self.inner.lock().expect("iterator lock");

        match iter.next() {
            Some(item) => {
                result.set("done", false)?;
                let value = <T as IntoJsValue<'js, E>>::into_js_value(item, self.ctx.clone());
                result.set("value", value)?;
            }
            None => {
                result.set("done", true)?;
                result.set("value", JsValue::undefined(self.ctx.clone()))?;
            }
        }

        Ok(result)
    }

    pub fn install_on(
        &self,
        _ctx: JsContext<'js, E>,
        _obj: &JsObject<'js, E>,
    ) -> JsResult<()>
    where
        E::Value: Send,
    {
        // TODO: HostCallback currently requires `F: 'static` while this iterator is tied to `'js`.
        // Revisit with engine-specific iterator registration or a non-`'static` callback channel.
        Ok(())
    }

    pub fn to_js_iterable(&self, ctx: JsContext<'js, E>) -> JsResult<JsObject<'js, E>> {
        let obj = JsObject::new(ctx.clone());
        self.install_on(ctx, &obj)?;
        Ok(obj)
    }
}

impl<'js, E, T> Clone for JsIterator<'js, E, T>
where
    E: JsEngine,
    E::Value: JsObjectOps + Send + 'static,
    T: IntoJsValue<'js, E> + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ctx: self.ctx.clone(),
        }
    }
}

pub trait IntoJsIteratorExt<'js, E: JsEngine, T>
where
    E::Value: JsObjectOps + Send + 'static,
{
    fn to_js_iter(self, ctx: JsContext<'js, E>) -> JsResult<JsObject<'js, E>>;
    fn install_js_iter(self, ctx: JsContext<'js, E>, obj: &JsObject<'js, E>) -> JsResult<()>;
}

impl<'js, E, T, I> IntoJsIteratorExt<'js, E, T> for I
where
    E: JsEngine + 'static,
    E::Value: JsObjectOps + Send + 'static,
    T: IntoJsValue<'js, E> + Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
    I::IntoIter: Send + 'static,
{
    fn to_js_iter(self, ctx: JsContext<'js, E>) -> JsResult<JsObject<'js, E>> {
        let js_iter = JsIterator::new(self, ctx.clone());
        js_iter.to_js_iterable(ctx)
    }

    fn install_js_iter(self, ctx: JsContext<'js, E>, obj: &JsObject<'js, E>) -> JsResult<()> {
        let js_iter = JsIterator::new(self, ctx.clone());
        js_iter.install_on(ctx, obj)
    }
}

pub fn install_iterator_symbol<'js, E: JsEngine>(
    _ctx: JsContext<'js, E>,
    _obj: &JsObject<'js, E>,
) -> JsResult<()>
where
    E::Value: Send,
{
    // TODO: same `'static` callback limitation as `JsIterator::install_on`.
    Ok(())
}
