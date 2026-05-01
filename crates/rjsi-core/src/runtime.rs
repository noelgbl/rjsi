use std::future::Future;

use crate::{Context, Engine, JsResult, ToJs};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MicrotaskDrainPolicy {
    Explicit,
    AfterEachCallback,
    Cooperative { budget: u32 },
}

pub trait Runtime<E: Engine> {
    fn with_scope<R>(&mut self, f: impl for<'rt> FnOnce(&mut Context<'rt, E>) -> R) -> R;
    fn microtask_policy(&self) -> MicrotaskDrainPolicy;
    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy);
}

pub trait PromiseBridge<E: Engine> {
    type AsyncError: Send + 'static;

    fn future_to_promise<'cx, F, T>(
        &mut self,
        cx: &mut Context<'cx, E>,
        fut: F,
    ) -> JsResult<'cx, E, E::Value<'cx>>
    where
        F: Future<Output = Result<T, Self::AsyncError>> + Send + 'static,
        T: for<'any> ToJs<'any, E> + Send + 'static;
}

pub trait LocalJsSpawn {
    fn spawn_local(&self, fut: impl Future<Output = ()> + 'static);
}

pub trait BlockingTaskPool {
    fn spawn_blocking<R, F>(
        &self,
        task: F,
    ) -> std::pin::Pin<Box<dyn Future<Output = R> + Send + 'static>>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static;
}
