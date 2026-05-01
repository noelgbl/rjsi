use rjsi::{DefaultRuntime, Runtime};

fn main() {
    let mut runtime = DefaultRuntime::default();

    let result: String = runtime.with_scope(|cx| {
        let value = cx.eval("'Hello from RJSI'").unwrap();
        value.to_string(cx).unwrap()
    });

    println!("{result}");
}
