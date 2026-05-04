use rjsi::{DefaultRuntime, Runtime};
use rjsi_console::init;

fn main() {
    let mut runtime = DefaultRuntime::default();

    runtime.with_scope(|cx| {
        init(cx).unwrap();

        cx.eval("console.log('Hello from RJSI');").unwrap();
    });
}
