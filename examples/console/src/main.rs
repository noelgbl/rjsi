use rjsi::{DefaultRuntime, Runtime, console};

fn main() {
    let mut runtime = DefaultRuntime::default();

    runtime.with_scope(|cx| {
        console::init(cx).unwrap();

        cx.eval("console.log('Hello from RJSI');").unwrap();
    });
}
