use crate::{Context, Engine};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MicrotaskDrainPolicy {
    Explicit,
    AfterEachCallback,
    Cooperative { budget: u32 },
}

pub trait Runtime<E: Engine> {
    fn with_scope<R>(&mut self, f: impl for<'js> FnOnce(&mut Context<'js, E>) -> R) -> R;
    fn microtask_policy(&self) -> MicrotaskDrainPolicy;
    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy);

    fn engine_name(&self) -> &'static str {
        E::ENGINE_NAME
    }
}
