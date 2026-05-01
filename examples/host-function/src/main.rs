use rjsi::DefaultRuntime;
use rjsi::{Args, CallbackCx, Engine, JsError, JsResult, Runtime, Value};

fn add<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'cx, E>,
    args: Args<'cx, E>,
) -> JsResult<'cx, E, Value<'cx, E>> {
    let a_val = args.get(0).ok_or(JsError::type_err("missing argument"))?;
    let b_val = args.get(1).ok_or(JsError::type_err("missing argument"))?;

    let a = a_val.to_f64(cx.cx())? as i32;
    let b = b_val.to_f64(cx.cx())? as i32;

    Ok(cx.cx().integer(a + b))
}

fn main() {
    let mut runtime = DefaultRuntime::new();

    runtime.with_scope(|cx| {
        let add = cx.function("add", add)?;

        let global = cx.globals();
        global.set(cx, "add", add.into_value())?;

        let out = cx.eval("add(20, 22);")?;
        let sum = out.to_f64(cx).unwrap() as i32;

        println!("add(20, 22) => {sum}");

        JsResult::Ok(())
    }).unwrap();
}
