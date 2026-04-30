use crate::{
    context::ContextLike,
    error::{EngineError, HostError, JsException},
    persistent::PersistentLike,
    scope::ScopeLike,
    value::{JsFunction, ValueLike},
};

pub trait Runtime: Sized + 'static {
    type Scope<'s, 'p: 's>: ScopeLike<'s, 'p, Self>;
    type Value<'s>: ValueLike<'s, Self>;
    type Function<'s>: JsFunction<'s, Self>;
    type Persistent: PersistentLike<Self>;
    type Context: ContextLike<Self>;
    type Error: std::error::Error
        + From<HostError>
        + From<JsException>
        + From<EngineError>
        + Send
        + Sync
        + 'static;

    fn name() -> &'static str;
    fn version() -> String;
}
