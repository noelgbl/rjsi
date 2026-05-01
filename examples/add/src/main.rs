use rjsi::quickjs::QuickJsRuntime;
use rjsi::Runtime;

fn main() {
    let mut runtime = QuickJsRuntime::new();
    let result: f64 = runtime.with(|cx| {
        let value = cx.eval("1 + 2 + 3").unwrap();
        value.to_f64(cx).unwrap()
    });

    println!("1 + 2 + 3 = {result}");
}
