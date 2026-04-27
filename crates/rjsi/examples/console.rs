use rjsi::quickjs::QuickJsError;

use rjsi::{ContextLike, ScopeLike};

fn main() -> Result<(), QuickJsError> {
    let runtime = rjsi::quickjs::QuickJsRuntimeContext::new();

    runtime.with_scope(|scope| {
        rjsi::console::init(scope)?;
        scope.eval("console.log('Hello, world!')")?;
        Ok(())
    })?;

    Ok(())
}
