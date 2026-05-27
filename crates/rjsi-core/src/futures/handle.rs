use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

use tokio::task::{JoinHandle, LocalSet};

use super::channel::{AsyncJsChannel, AsyncJsSender, PromiseId};
use crate::capabilities::{Microtasks, Promises};
use crate::{Context, Engine, Runtime};

pub struct RuntimeHandle<E: Engine + Promises + Microtasks> {
    inner: Rc<RefCell<E::Runtime>>,
    local_set: Rc<LocalSet>,
    channel: Rc<RefCell<AsyncJsChannel<E>>>,
    pending: Rc<Cell<usize>>,
}

impl<E: Engine + Promises + Microtasks> Clone for RuntimeHandle<E> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            local_set: Rc::clone(&self.local_set),
            channel: Rc::clone(&self.channel),
            pending: Rc::clone(&self.pending),
        }
    }
}

impl<E: Engine + Promises + Microtasks> RuntimeHandle<E> {
    pub(crate) fn new(
        inner: Rc<RefCell<E::Runtime>>,
        local_set: Rc<LocalSet>,
        channel: Rc<RefCell<AsyncJsChannel<E>>>,
        pending: Rc<Cell<usize>>,
    ) -> Self {
        Self {
            inner,
            local_set,
            channel,
            pending,
        }
    }

    pub fn with_scope<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Context<'_, E>) -> R,
    {
        let mut rt = self.inner.borrow_mut();
        rt.with_scope(f)
    }

    pub fn with_scope_and_pump<F, R>(&self, f: F) -> crate::Result<R>
    where
        F: FnOnce(&mut Context<'_, E>) -> crate::Result<R>,
    {
        let mut rt = self.inner.borrow_mut();
        let result = rt.with_scope(|cx| f(cx))?;
        let mut ch = self.channel.borrow_mut();
        rt.with_scope(|cx| ch.pump(cx))?;
        Ok(result)
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.local_set.spawn_local(future)
    }

    pub(crate) fn channel_sender(&self) -> AsyncJsSender<E> {
        self.channel.borrow().sender()
    }

    pub(crate) fn register_promise<'js>(
        &self,
        cx: &mut Context<'js, E>,
    ) -> crate::Result<(PromiseId, crate::Object<'js, E>)> {
        self.channel.borrow_mut().create_promise(cx)
    }

    pub(crate) fn increment_pending(&self) {
        self.pending.set(self.pending.get() + 1);
    }

    pub(crate) fn decrement_pending(&self) {
        let n = self.pending.get();
        if n > 0 {
            self.pending.set(n - 1);
        }
    }
}
