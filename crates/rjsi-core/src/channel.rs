use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::capabilities::{Microtasks, Promises};
use crate::context::{ContextMicrotaskExt, ContextPromiseExt};
use crate::convert::ToJs;
use crate::{Context, Engine, Object, PersistentValue, Result as RjsiResult};

pub type PromiseId = u64;

pub enum SettleMsg<T, Err> {
    Resolve(PromiseId, T),
    Reject(PromiseId, Err),
}

pub struct JsSender<T, Err> {
    tx: Sender<SettleMsg<T, Err>>,
}

impl<T, Err> Clone for JsSender<T, Err> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T, Err> JsSender<T, Err> {
    pub fn resolve(
        &self,
        id: PromiseId,
        value: T,
    ) -> Result<(), mpsc::SendError<SettleMsg<T, Err>>> {
        self.tx.send(SettleMsg::Resolve(id, value))
    }

    pub fn reject(
        &self,
        id: PromiseId,
        reason: Err,
    ) -> Result<(), mpsc::SendError<SettleMsg<T, Err>>> {
        self.tx.send(SettleMsg::Reject(id, reason))
    }
}

pub struct JsChannel<E: Engine + Promises, T, Err> {
    rx: Receiver<SettleMsg<T, Err>>,
    next_id: PromiseId,
    resolvers: HashMap<PromiseId, PersistentValue<E>>,
}

impl<E: Engine + Promises, T, Err> JsChannel<E, T, Err> {
    pub fn new() -> (JsSender<T, Err>, Self) {
        let (tx, rx) = mpsc::channel();
        (
            JsSender { tx },
            Self {
                rx,
                next_id: 0,
                resolvers: HashMap::new(),
            },
        )
    }

    pub fn create_promise<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
    ) -> RjsiResult<(PromiseId, Object<'js, E>)> {
        let (promise, resolver) = cx.promise_new()?;
        let id = self.next_id;
        self.next_id += 1;
        self.resolvers
            .insert(id, cx.persist_value(resolver.into_value()));
        Ok((id, promise))
    }

    pub fn pump<'js, F, G>(
        &mut self,
        cx: &mut Context<'js, E>,
        mut map_resolve: F,
        mut map_reject: G,
    ) -> RjsiResult<()>
    where
        F: FnMut(&mut Context<'js, E>, T) -> RjsiResult<crate::Value<'js, E>>,
        G: FnMut(&mut Context<'js, E>, Err) -> RjsiResult<crate::Value<'js, E>>,
    {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                SettleMsg::Resolve(id, val) => {
                    if let Some(resolver) = self.resolvers.remove(&id) {
                        let resolver_obj = resolver.restore(cx)?.try_as_object()?;
                        let js_val = map_resolve(cx, val)?;
                        cx.promise_resolve(resolver_obj, js_val)?;
                    }
                }
                SettleMsg::Reject(id, err) => {
                    if let Some(resolver) = self.resolvers.remove(&id) {
                        let resolver_obj = resolver.restore(cx)?.try_as_object()?;
                        let js_err = map_reject(cx, err)?;
                        cx.promise_reject(resolver_obj, js_err)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn settle<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
        id: PromiseId,
        outcome: std::result::Result<crate::Value<'js, E>, crate::Value<'js, E>>,
    ) -> RjsiResult<()> {
        if let Some(resolver) = self.resolvers.remove(&id) {
            let resolver_obj = resolver.restore(cx)?.try_as_object()?;
            match outcome {
                Ok(value) => cx.promise_resolve(resolver_obj, value)?,
                Err(reason) => cx.promise_reject(resolver_obj, reason)?,
            }
        }
        Ok(())
    }

    pub fn pump_to_js<'js>(&mut self, cx: &mut Context<'js, E>) -> RjsiResult<()>
    where
        T: ToJs<'js, E>,
        Err: ToJs<'js, E>,
    {
        self.pump(cx, |cx, val| val.to_js(cx), |cx, err| err.to_js(cx))
    }
}

impl<E: Engine + Promises + Microtasks, T, Err> JsChannel<E, T, Err> {
    pub fn pump_and_drain_to_js<'js>(&mut self, cx: &mut Context<'js, E>) -> RjsiResult<()>
    where
        T: ToJs<'js, E>,
        Err: ToJs<'js, E>,
    {
        self.pump_to_js(cx)?;
        cx.drain_microtasks();
        Ok(())
    }
}
