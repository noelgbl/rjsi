use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::capabilities::{Microtasks, Promises};
use crate::context::{ContextMicrotaskExt, ContextPromiseExt};
use crate::convert::ToJs;
use crate::{Context, Engine, Object, Result as RjsiResult};

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

pub struct JsChannel<'rt, E: Engine + Promises, T, Err> {
    rx: Receiver<SettleMsg<T, Err>>,
    next_id: PromiseId,
    resolvers: HashMap<PromiseId, E::PromiseResolver<'rt>>,
}

impl<'rt, E: Engine + Promises, T, Err> JsChannel<'rt, E, T, Err> {
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

    pub fn create_promise(
        &mut self,
        cx: &mut Context<'rt, E>,
    ) -> RjsiResult<(PromiseId, Object<'rt, E>)> {
        let (promise, resolver) = cx.promise_new()?;
        let id = self.next_id;
        self.next_id += 1;
        self.resolvers.insert(id, resolver);
        Ok((id, promise))
    }

    pub fn pump<F, G>(
        &mut self,
        cx: &mut Context<'rt, E>,
        mut map_resolve: F,
        mut map_reject: G,
    ) -> RjsiResult<()>
    where
        F: FnMut(&mut Context<'rt, E>, T) -> RjsiResult<crate::Value<'rt, E>>,
        G: FnMut(&mut Context<'rt, E>, Err) -> RjsiResult<crate::Value<'rt, E>>,
    {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                SettleMsg::Resolve(id, val) => {
                    if let Some(resolver) = self.resolvers.remove(&id) {
                        let js_val = map_resolve(cx, val)?;
                        cx.promise_resolve(resolver, js_val)?;
                    }
                }
                SettleMsg::Reject(id, err) => {
                    if let Some(resolver) = self.resolvers.remove(&id) {
                        let js_err = map_reject(cx, err)?;
                        cx.promise_reject(resolver, js_err)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn pump_to_js(&mut self, cx: &mut Context<'rt, E>) -> RjsiResult<()>
    where
        T: ToJs<'rt, E>,
        Err: ToJs<'rt, E>,
    {
        self.pump(cx, |cx, val| val.to_js(cx), |cx, err| err.to_js(cx))
    }
}

impl<'rt, E: Engine + Promises + Microtasks, T, Err> JsChannel<'rt, E, T, Err> {
    pub fn pump_and_drain_to_js(&mut self, cx: &mut Context<'rt, E>) -> RjsiResult<()>
    where
        T: ToJs<'rt, E>,
        Err: ToJs<'rt, E>,
    {
        self.pump_to_js(cx)?;
        cx.drain_microtasks();
        Ok(())
    }
}
