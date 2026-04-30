use rjsi::{ContextLike, Error, ScopeLike, ValueExt};

fn main() -> Result<(), Error> {
    let runtime = rjsi::quickjs::QuickJsRuntimeContext::new();
    let result: i32 = runtime.with_scope(|scope| {
        let value = scope.eval("1 + 2 + 3")?;
        value.coerce(scope)
    })?;

    println!("1 + 2 + 3 = {result}");

    Ok(())
}
