use rjsi::quickjs::QuickJsRuntimeContext;
use rjsi::{ContextLike, Error, ScopeLike, ValueExt};

fn main() -> Result<(), Error> {
    let runtime = QuickJsRuntimeContext::new();

    let result: String = runtime.with_scope(|scope| {
        let value = scope.eval("'Hello from QuickJS via RJSI'")?;
        value.coerce(scope)
    })?;

    println!("{result}");

    Ok(())
}
