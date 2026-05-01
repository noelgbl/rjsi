use crate::{Context, Engine, JsResult};

pub trait FromJs<'cx, E: Engine>: Sized {
    fn from_js(cx: &mut Context<'_, E>, value: E::Value<'cx>) -> JsResult<'cx, E, Self>;
}

pub trait ToJs<'cx, E: Engine> {
    fn to_js(self, cx: &mut Context<'_, E>) -> JsResult<'cx, E, E::Value<'cx>>;
}

#[cfg(feature = "serde")]
mod serde_layer {
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    use super::*;

    pub trait ToJsSerde<'cx, E: Engine>: Serialize {
        fn to_js_serde(self, cx: &mut Context<'_, E>) -> JsResult<'cx, E, E::Value<'cx>>;
    }

    pub trait FromJsSerde<'cx, E: Engine>: DeserializeOwned {
        fn from_js_serde(cx: &mut Context<'_, E>, value: E::Value<'cx>) -> JsResult<'cx, E, Self>;
    }
}

#[cfg(feature = "serde")]
pub use serde_layer::*;
