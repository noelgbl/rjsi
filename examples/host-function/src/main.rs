use rjsi::quickjs::QuickJsRuntime;
use rjsi::{Runtime, Value, __cx};
use rquickjs::Function;

fn main() {
    let mut runtime = QuickJsRuntime::new();
    runtime.with(|cx| {
        // In `rjsi`, you can access the underlying engine context to perform
        // engine-specific operations, like creating host functions.
        let raw_cx = __cx::context_mut(cx).clone();

        // Use `rquickjs` native functionality to create a closure-based host function.
        let add = Function::new(raw_cx, |a: i32, b: i32| -> i32 {
            a + b
        }).unwrap();

        // Convert the engine-specific function into a generic `rjsi` Value.
        let val = add.into_value();
        let rjsi_val = Value::new(unsafe { std::mem::transmute(val) });

        let global = cx.globals();
        global.set(cx, "addHost", rjsi_val).unwrap();

        let out = cx.eval("addHost(20, 22);").unwrap();
        let sum = out.to_f64(cx).unwrap() as i32;

        assert_eq!(sum, 42);
        println!("addHost(20, 22) => {sum}");
    });
}
