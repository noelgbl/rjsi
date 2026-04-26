use rjsi::{JsRuntime, JsScope, Source};

fn main() -> rjsi::JsResult<()> {
    let runtime = rjsi::v8::V8RuntimeContext::new();

    runtime.with_scope(|scope| {
        rjsi::console::init(scope)?;
        scope.eval(Source::from_bytes("console.log('Hello, world!')"))?;
        Ok(())
    })?;

    Ok(())
}
