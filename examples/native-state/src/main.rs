//! Attach Rust data to a JS object and call from JavaScript into Rust that
//! mutates it.
use rjsi::{
    Args, Context, ContextNativeStateExt, DefaultRuntime, Engine, Error, NativeState, NativeStateSupport, Result, Runtime, Value
};

struct Counter {
    value: i32,
}

impl NativeState for Counter {}

fn increment<'rt, E: Engine + NativeStateSupport>(
    cx: &mut Context<'rt, E>,
    this: Value<'rt, E>,
    _args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    let mut obj = this.try_as_object()?;

    let c = obj
        .native_state_mut::<Counter>(cx)
        .ok_or_else(|| Error::type_err("increment: expected Counter native state"))?;
    c.value += 1;

    Ok(cx.number(f64::from(c.value)))
}

fn main() {
    let mut runtime = DefaultRuntime::default();

    runtime.with_scope(|cx| {
        let obj = cx.with_state(Counter { value: 0 }).unwrap();

        let increment = cx.function("increment", increment).unwrap();
        obj.set(cx, "increment", increment.into_value()).unwrap();

        cx.globals()
            .set(cx, "counterObj", obj.into_value())
            .unwrap();

        cx.eval(
            r#"
            globalThis.counterObj.increment();
        "#,
        )
        .unwrap();

        let holder_val = cx.globals().get(cx, "counterObj").unwrap();
        let holder_obj = holder_val.try_as_object().unwrap();

        assert_eq!(holder_obj.native_state::<Counter>(cx).unwrap().value, 1);

        cx.eval(
            r#"
            globalThis.counterObj.increment();
        "#,
        )
        .unwrap();

        assert_eq!(holder_obj.native_state::<Counter>(cx).unwrap().value, 2);

        println!(
            "Native counter after 2 increments from JS: {}",
            holder_obj.native_state::<Counter>(cx).unwrap().value
        );
    });
}
