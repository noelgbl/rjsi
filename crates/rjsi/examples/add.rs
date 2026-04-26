use rjsi::{FromJs, JsRuntime, JsScope, Source};

fn main() -> rjsi::JsResult<()> {
    let runtime = rjsi::v8::V8RuntimeContext::new();
    let result = runtime.with_scope(|scope| {
        let value = scope.eval(Source::from_bytes("1 + 2 + 3"))?;
        i32::from_js(scope, &value)
    })?;

    println!("1 + 2 + 3 = {result}");

    Ok(())
}
