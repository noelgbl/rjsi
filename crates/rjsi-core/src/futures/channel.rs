use std::collections::HashMap;

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::capabilities::{Microtasks, Promises};
use crate::context::{ContextMicrotaskExt, ContextPromiseExt};
use crate::{Context, Engine, PersistentValue, Result as RjsiResult};

pub type PromiseId = u64;

pub enum AsyncSettleMsg<E: Engine> {
    Resolve(PromiseId, PersistentValue<E>),
    Reject(PromiseId, PersistentValue<E>),
}

#[derive(Clone)]
pub struct AsyncJsSender<E: Engine> {
    tx: UnboundedSender<AsyncSettleMsg<E>>,
}

impl<E: Engine> AsyncJsSender<E> {
    pub fn resolve(&self, id: PromiseId, value: PersistentValue<E>) {
        let _ = self.tx.send(AsyncSettleMsg::Resolve(id, value));
    }

    pub fn reject(&self, id: PromiseId, reason: PersistentValue<E>) {
        let _ = self.tx.send(AsyncSettleMsg::Reject(id, reason));
    }
}

pub struct AsyncJsChannel<E: Engine + Promises> {
    rx: UnboundedReceiver<AsyncSettleMsg<E>>,
    tx: UnboundedSender<AsyncSettleMsg<E>>,
    next_id: PromiseId,
    resolvers: HashMap<PromiseId, PersistentValue<E>>,
}

impl<E: Engine + Promises> AsyncJsChannel<E> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            rx,
            tx,
            next_id: 0,
            resolvers: HashMap::new(),
        }
    }

    pub fn sender(&self) -> AsyncJsSender<E> {
        AsyncJsSender {
            tx: self.tx.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.resolvers.is_empty()
    }

    pub fn create_promise<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
    ) -> RjsiResult<(PromiseId, crate::Object<'js, E>)> {
        let (promise, resolver) = cx.promise_new()?;
        let id = self.next_id;
        self.next_id += 1;
        self.resolvers
            .insert(id, cx.persist_value(resolver.into_value()));
        Ok((id, promise))
    }
}

impl<E: Engine + Promises + Microtasks> AsyncJsChannel<E> {
    pub fn pump<'js>(&mut self, cx: &mut Context<'js, E>) -> RjsiResult<()> {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AsyncSettleMsg::Resolve(id, value) => {
                    if let Some(resolver) = self.resolvers.remove(&id) {
                        let resolver_obj = resolver.restore(cx)?.try_as_object()?;
                        let js_val = value.restore(cx)?;
                        cx.promise_resolve(resolver_obj, js_val)?;
                    }
                }
                AsyncSettleMsg::Reject(id, reason) => {
                    if let Some(resolver) = self.resolvers.remove(&id) {
                        let resolver_obj = resolver.restore(cx)?.try_as_object()?;
                        let js_err = reason.restore(cx)?;
                        cx.promise_reject(resolver_obj, js_err)?;
                    }
                }
            }
        }
        cx.drain_microtasks();
        Ok(())
    }
}

impl<E: Engine + Promises> Default for AsyncJsChannel<E> {
    fn default() -> Self {
        Self::new()
    }
}
