use crate::{Context, Engine, Object, Result, Value};

#[repr(transparent)]
pub struct Function<'cx, E: Engine> {
    pub(crate) raw: E::Function<'cx>,
}

impl<'cx, E: Engine> Function<'cx, E> {
    pub fn new(raw: E::Function<'cx>) -> Self {
        Self { raw }
    }

    pub fn into_raw(self) -> E::Function<'cx> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Function<'cx> {
        &self.raw
    }

    pub fn call(
        &self,
        cx: &mut Context<'cx, E>,
        this: Value<'cx, E>,
        args: &[Value<'cx, E>],
    ) -> Result<Value<'cx, E>> {
        let raw_args: &[E::Value<'cx>] = unsafe {
            std::slice::from_raw_parts(args.as_ptr() as *const E::Value<'cx>, args.len())
        };

        E::function_call(&mut cx.raw, &self.raw, this.raw, raw_args).map(Value::new)
    }

    pub fn call_no_args(&self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        let this: Value<'cx, E> = Value::new(E::make_undefined(&mut cx.raw));
        E::function_call(&mut cx.raw, &self.raw, this.raw, &[]).map(Value::new)
    }

    pub fn into_value(self) -> Value<'cx, E> {
        Value::new(E::function_to_value(self.raw))
    }

    pub fn into_object(self) -> Object<'cx, E> {
        Object::new(E::function_to_object(self.raw))
    }
}
