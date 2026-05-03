use crate::{Context, Engine};

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
