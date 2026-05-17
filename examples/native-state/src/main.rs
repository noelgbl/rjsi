//! Attach Rust data to a JS object and call from JavaScript into Rust that
//! mutates it.
use rjsi::{Context, ContextNativeStateExt, DefaultRuntime, NativeState, Runtime};

struct Counter {
    value: i32,
}

impl NativeState for Counter {}

fn increment<'rt, E: rjsi::Engine>(
    counter: &mut Counter,
    increment: i32,
) -> f64 {
    counter.value += increment;
    counter.value as f64
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

        cx.eval("globalThis.counterObj.increment(5);").unwrap();

        let holder_val = cx.globals().get(cx, "counterObj").unwrap();
        let holder_obj = holder_val.try_as_object().unwrap();

        assert_eq!(holder_obj.native_state::<Counter>(cx).unwrap().value, 5);

        cx.eval("globalThis.counterObj.increment(10);").unwrap();

        assert_eq!(holder_obj.native_state::<Counter>(cx).unwrap().value, 15);

        println!(
            "Native counter after 2 increments from JS: {}",
            holder_obj.native_state::<Counter>(cx).unwrap().value
        );
    });
}
