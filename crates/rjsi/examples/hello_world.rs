use rjsi::{JsRuntime, JsScope, Source};

fn main() -> rjsi::JsResult<()> {
    let runtime = rjsi::v8::V8RuntimeContext::new();
    runtime.with_scope(|scope| {
        let value = scope.eval(Source::from_bytes("'Hello from V8 via RJSI'"))?;
        println!("type: {}", scope.value_type(&value));
        Ok(())
    })
}
