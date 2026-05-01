use rjsi::{Args, CallbackCx, DefaultRuntime, Engine, JsError, JsResult, Runtime, Value};

fn add<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<'rt, E, Value<'rt, E>> {
    let a_val = args.get(0).ok_or(JsError::type_err("missing argument"))?;
    let b_val = args.get(1).ok_or(JsError::type_err("missing argument"))?;

    let a = a_val.to_f64(cx.cx())? as i32;
    let b = b_val.to_f64(cx.cx())? as i32;

    Ok(cx.cx().integer(a + b))
}

fn main() {
    let mut runtime = DefaultRuntime::new();

    let result = runtime
        .with_scope(|cx| {
            let add = cx.function("add", add).unwrap();

            let global = cx.globals();
            global.set(cx, "add", add.into_value()).unwrap();

            let out = cx.eval("add(20, 22);").unwrap();
            out.to_f64(cx).unwrap() as i32
        });

    println!("add(20, 22) => {result}");
}
