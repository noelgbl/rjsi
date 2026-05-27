use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

use tokio::task::{JoinHandle, LocalSet};

use super::channel::AsyncJsChannel;
use super::handle::RuntimeHandle;
use crate::capabilities::{Microtasks, Promises};
use crate::{Context, Engine, Result, Runtime};

pub struct AsyncRuntime<E: Engine + Promises + Microtasks> {
    inner: Rc<RefCell<E::Runtime>>,
    local_set: Rc<LocalSet>,
    channel: Rc<RefCell<AsyncJsChannel<E>>>,
    pending: Rc<Cell<usize>>,
}

impl<E: Engine + Promises + Microtasks> AsyncRuntime<E>
where
    E::Runtime: Default,
{
    pub fn new() -> Self {
        Self::from_runtime(E::Runtime::default())
    }
}

impl<E: Engine + Promises + Microtasks> AsyncRuntime<E> {
    pub fn from_runtime(rt: E::Runtime) -> Self {
        Self {
            inner: Rc::new(RefCell::new(rt)),
            local_set: Rc::new(LocalSet::new()),
            channel: Rc::new(RefCell::new(AsyncJsChannel::new())),
            pending: Rc::new(Cell::new(0)),
        }
    }

    pub fn handle(&self) -> RuntimeHandle<E> {
        RuntimeHandle::new(
            Rc::clone(&self.inner),
            Rc::clone(&self.local_set),
            Rc::clone(&self.channel),
            Rc::clone(&self.pending),
        )
    }

    pub fn engine_name(&self) -> &'static str {
        E::ENGINE_NAME
    }

    pub async fn with_scope<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Context<'_, E>) -> Result<R>,
    {
        let result = self
            .local_set
            .run_until(async {
                let mut rt = self.inner.borrow_mut();
                let r = rt.with_scope(|cx| f(cx));
                let mut ch = self.channel.borrow_mut();
                rt.with_scope(|cx| ch.pump(cx))?;
                r
            })
            .await;
        tokio::task::yield_now().await;
        result
    }

    pub async fn idle(&self) -> Result<()> {
        loop {
            self.local_set.run_until(tokio::task::yield_now()).await;

            {
                let mut rt = self.inner.borrow_mut();
                let mut ch = self.channel.borrow_mut();
                rt.with_scope(|cx| ch.pump(cx))?;
            }

            let pending = self.pending.get();
            let channel_empty = self.channel.borrow().is_empty();
            if pending == 0 && channel_empty {
                return Ok(());
            }
        }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.local_set.spawn_local(future)
    }
}

impl<E: Engine + Promises + Microtasks> Default for AsyncRuntime<E>
where
    E::Runtime: Default,
{
    fn default() -> Self {
        Self::new()
    }
}
