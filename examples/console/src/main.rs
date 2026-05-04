use rjsi::{DefaultRuntime, Runtime, console};

fn main() {
    let mut runtime = DefaultRuntime::default();

    runtime.with_scope(|cx| {
        console::init(cx).unwrap();

        cx.eval("console.log('Hello from RJSI');").unwrap();
        cx.eval("console.log('%s %s', 'fmt', 'args', 'and', 'rest', 2, 0);")
            .unwrap();
    });
}
