use crate::{Context, Engine, Result};

/// Engines that expose native Promise primitives.
pub trait Promises: Engine {
    /// The handle used to resolve or reject a promise.
    type PromiseResolver<'cx>: 'cx;

    /// Creates a new native Promise.
    fn promise_new<'rt>(
        cx: &mut Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::PromiseResolver<'rt>)>;

    /// Resolves a promise.
    fn promise_resolve<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::PromiseResolver<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()>;

    /// Rejects a promise.
    fn promise_reject<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::PromiseResolver<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()>;
}

/// Engines that allow manual manipulation of the microtask queue.
pub trait Microtasks: Engine {
    /// Enqueues a function to be run as a microtask.
    fn queue_microtask<'rt>(cx: &mut Context<'rt, Self>, task: Self::Function<'rt>);

    /// Drains the microtask queue synchronously.
    fn drain_microtasks<'rt>(cx: &mut Context<'rt, Self>);
}
