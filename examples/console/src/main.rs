use rjsi::{DefaultRuntime, Runtime, console};

fn main() {
    let mut runtime = DefaultRuntime::default();
    let engine_name = runtime.engine_name();

    runtime.with_scope(|cx| {
        console::init(cx).unwrap();

        cx.eval(&format!("console.log('Hello from {}');", engine_name))
            .unwrap();

        cx.eval("console.log('%s %s', 'fmt', 'args', 'and', 'rest', 2, 0);")
            .unwrap();
    });
}
