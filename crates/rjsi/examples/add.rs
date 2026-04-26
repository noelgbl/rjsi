use rjsi::quickjs::QuickJsError;

use rjsi::{ContextLike, ScopeLike, ValueLike};

fn main() -> Result<(), QuickJsError> {
    let runtime = rjsi::quickjs::QuickJsRuntimeContext::new();
    let result = runtime.with_scope(|scope| {
        let value = scope.eval("1 + 2 + 3")?;
        value
            .as_i32(scope)
            .ok_or_else(|| {
                QuickJsError::from(rjsi::HostError::type_error(
                    rjsi::E_TYPE,
                    "expected integer",
                ))
            })
    })?;

    println!("1 + 2 + 3 = {result}");

    Ok(())
}
