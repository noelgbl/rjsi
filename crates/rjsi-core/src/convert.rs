use crate::{Context, Engine, JsResult};

pub trait FromJs<'cx, E: Engine>: Sized {
    fn from_js(cx: &mut Context<'cx, E>, value: E::Value<'cx>) -> JsResult<'cx, E, Self>;
}

pub trait ToJs<'cx, E: Engine> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> JsResult<'cx, E, E::Value<'cx>>;
}
