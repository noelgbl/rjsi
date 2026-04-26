use rjsi::{FromJs, JsRuntime, JsScope, Source};

fn main() -> rjsi::JsResult<()> {
    let runtime = rjsi::v8::V8RuntimeContext::new();
    let result = runtime.with_scope(|scope| {
        let value = scope.eval(Source::from_bytes("'Hello from V8 via RJSI'"))?;
        String::from_js(scope, &value)
    })?;

    println!("{result}");

    Ok(())
}
