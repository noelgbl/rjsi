use std::future::Future;
use std::rc::Rc;

use super::handle::RuntimeHandle;
use crate::capabilities::{Microtasks, Promises};
use crate::{Args, Context, Engine, Function, PersistentValue, RawHostFn, Result, ToJs, Value};

pub struct AsyncArgs<E: Engine> {
    args: Vec<PersistentValue<E>>,
}

impl<E: Engine> AsyncArgs<E> {
    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&PersistentValue<E>> {
        self.args.get(index)
    }

    pub fn into_vec(self) -> Vec<PersistentValue<E>> {
        self.args
    }
}

pub trait ContextAsyncExt<'js, E: Engine + Promises + Microtasks> {
    fn async_function<F, Fut, R>(
        &mut self,
        handle: &RuntimeHandle<E>,
        name: &str,
        f: F,
    ) -> Result<Function<'js, E>>
    where
        F: Fn(RuntimeHandle<E>, AsyncArgs<E>) -> Fut + 'static,
        Fut: Future<Output = Result<R>> + 'static,
        R: for<'r> ToJs<'r, E> + 'static;
}

impl<'js, E: Engine + Promises + Microtasks> ContextAsyncExt<'js, E> for Context<'js, E> {
    fn async_function<F, Fut, R>(
        &mut self,
        handle: &RuntimeHandle<E>,
        name: &str,
        f: F,
    ) -> Result<Function<'js, E>>
    where
        F: Fn(RuntimeHandle<E>, AsyncArgs<E>) -> Fut + 'static,
        Fut: Future<Output = Result<R>> + 'static,
        R: for<'r> ToJs<'r, E> + 'static,
    {
        let f = Rc::new(f);
        let handle = handle.clone();
        let adapter = AsyncFnAdapter::<E, F, Fut, R> {
            f,
            handle,
            _p: std::marker::PhantomData,
        };
        self.raw_function(name, adapter)
    }
}

struct AsyncFnAdapter<E, F, Fut, R>
where
    E: Engine + Promises + Microtasks,
    F: Fn(RuntimeHandle<E>, AsyncArgs<E>) -> Fut + 'static,
    Fut: Future<Output = Result<R>> + 'static,
    R: for<'r> ToJs<'r, E> + 'static,
{
    f: Rc<F>,
    handle: RuntimeHandle<E>,
    _p: std::marker::PhantomData<fn() -> (Fut, R)>,
}

impl<E, F, Fut, R> RawHostFn<E> for AsyncFnAdapter<E, F, Fut, R>
where
    E: Engine + Promises + Microtasks,
    F: Fn(RuntimeHandle<E>, AsyncArgs<E>) -> Fut + 'static,
    Fut: Future<Output = Result<R>> + 'static,
    R: for<'r> ToJs<'r, E> + 'static,
{
    fn call<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
        _this: Value<'js, E>,
        args: Args<'js, E>,
    ) -> Result<Value<'js, E>> {
        let mut persisted = Vec::with_capacity(args.len());
        for i in 0..args.len() {
            let v = args
                .get(i)
                .ok_or_else(|| crate::Error::type_err("missing arg"))?;
            persisted.push(cx.persist_value(v));
        }
        let async_args = AsyncArgs { args: persisted };

        let (id, promise_obj) = self.handle.register_promise(cx)?;
        let sender = self.handle.channel_sender();

        let f = Rc::clone(&self.f);
        let handle = self.handle.clone();
        let pending_handle = self.handle.clone();

        handle.increment_pending();
        self.handle.spawn(async move {
            let outcome = (f)(handle.clone(), async_args).await;
            let settle = handle.with_scope(|cx| match outcome {
                Ok(value) => match value.to_js(cx) {
                    Ok(v) => SettleEnvelope::Resolve(cx.persist_value(v)),
                    Err(e) => reject_with_error(cx, &e.to_string()),
                },
                Err(e) => reject_with_error(cx, &e.to_string()),
            });

            match settle {
                SettleEnvelope::Resolve(v) => sender.resolve(id, v),
                SettleEnvelope::Reject(v) => sender.reject(id, v),
            }
            pending_handle.decrement_pending();
        });

        Ok(promise_obj.into_value())
    }
}

enum SettleEnvelope<E: Engine> {
    Resolve(PersistentValue<E>),
    Reject(PersistentValue<E>),
}

fn reject_with_error<'js, E: Engine>(cx: &mut Context<'js, E>, msg: &str) -> SettleEnvelope<E> {
    if let Ok(s) = cx.string(msg) {
        SettleEnvelope::Reject(cx.persist_value(s))
    } else {
        let und = cx.undefined();
        SettleEnvelope::Reject(cx.persist_value(und))
    }
}
