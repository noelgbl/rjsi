use rjsi::{DefaultRuntime, Runtime};

fn add(a: f64, b: f64) -> f64 {
    a + b
}

fn main() {
    let mut runtime = DefaultRuntime::default();

    let result = runtime.with_scope(|cx| {
        let add = cx.function("add", add).unwrap();

        let global = cx.globals();
        global.set(cx, "add", add.into_value()).unwrap();

        let out = cx.eval("add(20, 22);").unwrap();
        out.to_f64(cx).unwrap() as i32
    });

    println!("add(20, 22) => {result}");
}
