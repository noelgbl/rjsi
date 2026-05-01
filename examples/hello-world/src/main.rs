use rjsi::Runtime;
use rjsi::quickjs::QuickJsRuntime;

fn main() {
    let mut runtime = QuickJsRuntime::new();

    let result: String = runtime.with(|cx| {
        let value = cx.eval("'Hello from QuickJS via RJSI'").unwrap();
        value.to_string(cx).unwrap()
    });

    println!("{result}");
}
