use rjsi::{DefaultRuntime, Runtime};

fn main() {
    let mut runtime = DefaultRuntime::new();

    let result = runtime.with_scope(|cx| {
        let value = cx.eval("1 + 2 + 3").unwrap();
        value.to_f64(cx).unwrap()
    });

    println!("1 + 2 + 3 = {result}");
}
