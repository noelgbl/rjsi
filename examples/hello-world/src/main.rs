use rjsi::{DefaultRuntime, Runtime};

fn main() {
    let mut runtime = DefaultRuntime::default();
    let engine_name = runtime.engine_name();

    let result: String = runtime.with_scope(|cx| {
        let value = cx.eval(&format!("'Hello from {}';", engine_name)).unwrap();
        value.to_string(cx).unwrap()
    });

    println!("{result}");
}
