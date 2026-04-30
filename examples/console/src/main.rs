use rjsi::quickjs::QuickJsRuntimeContext;
use rjsi::{ContextLike, Error, ScopeLike, console};

fn main() -> Result<(), Error> {
    let runtime = QuickJsRuntimeContext::new();

    runtime.with_scope(|scope| {
        console::init::<rjsi::quickjs::QuickJsRuntime>(scope)?;
        scope.eval("console.log('Hello, world!')")?;
        Ok(())
    })?;

    Ok(())
}
