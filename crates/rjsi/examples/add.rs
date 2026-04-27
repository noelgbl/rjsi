use rjsi::quickjs::QuickJsError;

use rjsi::{ContextLike, ScopeLike, ValueLike, ValueExt};

fn main() -> Result<(), QuickJsError> {
    let runtime = rjsi::quickjs::QuickJsRuntimeContext::new();
    let result = runtime.with_scope(|scope| {
        let value = scope.eval("1 + 2 + 3")?;
        value
            .coerce::<i32>(scope)
    })?;

    println!("1 + 2 + 3 = {result}");

    Ok(())
}
