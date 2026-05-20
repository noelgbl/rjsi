use crate::{Context, Engine, Object, Result};

/// The execution state of a promise.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum PromiseState {
    /// The promise has not yet completed.
    Pending,
    /// The promise completed succefully.
    Resolved,
    /// The promise completed with an error.
    Rejected,
}

/// A JavaScript promise.
#[repr(transparent)]
pub struct Promise<'js, E: Engine>(pub(crate) Object<'js, E>);

impl<'js, E: Engine> Promise<'js, E> {
    pub fn new(obj: Object<'js, E>) -> Self {
        Self(obj)
    }

    pub fn into_object(self) -> Object<'js, E> {
        self.0
    }

    pub fn as_object(&self) -> &Object<'js, E> {
        &self.0
    }
}

pub trait Promises: Engine {
    fn promise_new<'rt>(
        cx: &mut Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::Object<'rt>)>;

    fn promise_resolve<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()>;

    fn promise_reject<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()>;

    fn promise_state<'rt>(
        cx: &mut Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<PromiseState>;

    fn promise_result<'rt>(
        cx: &mut Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<Option<std::result::Result<Self::Value<'rt>, Self::Value<'rt>>>>;
}

/// Engines that allow manual manipulation of the microtask queue.
pub trait Microtasks: Engine {
    /// Enqueues a function to be run as a microtask.
    fn queue_microtask<'rt>(cx: &mut Context<'rt, Self>, task: Self::Function<'rt>);

    /// Drains the microtask queue synchronously.
    fn drain_microtasks<'rt>(cx: &mut Context<'rt, Self>);
}

pub trait Modules: Engine + Promises {
    fn install_module_host(
        runtime: &mut Self::Runtime,
        host: crate::module::ModuleHost,
    ) -> Result<()>;

    fn set_import_meta_hook(
        runtime: &mut Self::Runtime,
        hook: crate::module::ImportMetaHook,
    ) -> Result<()>;

    fn module_evaluate<'rt>(
        cx: &mut Context<'rt, Self>,
        name: &str,
        src: &str,
    ) -> Result<Self::Object<'rt>>;

    fn module_import<'rt>(
        cx: &mut Context<'rt, Self>,
        specifier: &str,
    ) -> Result<Self::Object<'rt>>;
}
