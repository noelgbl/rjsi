use rjsi::quickjs::{QuickJsError, QuickJsRuntimeContext};
use rjsi::{ContextLike, FromJs, ScopeLike, ValueLike};

fn main() -> Result<(), QuickJsError> {
    let runtime = QuickJsRuntimeContext::new();

    let result: String = runtime.with_scope(|scope| {
        let value = scope.eval("'Hello from QuickJS via RJSI'")?;
        let value = String::from_js(scope, value)?;

        Ok(value)
    })?;

    println!("{result}");

    Ok(())
}
