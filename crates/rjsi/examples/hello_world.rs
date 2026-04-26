use rjsi::quickjs::QuickJsError;

use rjsi::{ContextLike, ScopeLike, ValueLike};

fn main() -> Result<(), QuickJsError> {
    let runtime = rjsi::quickjs::QuickJsRuntimeContext::new();
    let result = runtime.with_scope(|scope| {
        let value = scope.eval("'Hello from QuickJS via RJSI'")?;
        value
            .with_str(scope, str::to_owned)
            .ok_or_else(|| {
                QuickJsError::from(rjsi::HostError::type_error(
                    rjsi::E_TYPE,
                    "expected string",
                ))
            })
    })?;

    println!("{result}");

    Ok(())
}
