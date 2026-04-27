//! Register a **native (host) function** with the full `Args` API: pull typed
//! values from `arguments` by index and return a new JS value.
//!
//! Run: `cargo run --example host_function --features quickjs`

use rjsi::quickjs::{QuickJsError, QuickJsRuntimeContext, QuickJsValue};
use rjsi::{ContextLike, ScopeLike, ValueLike};

fn main() -> Result<(), QuickJsError> {
    let runtime = QuickJsRuntimeContext::new();
    runtime.with_scope(|scope| {
        // `function` needs a `Send` + `'static` closure, so the callback cannot
        // close over the outer stack — only `scope` and `args` for each call.
        let add: QuickJsValue<'_> = scope.function(
            |scope, args: rjsi::Args<'_, rjsi::quickjs::QuickJsRuntime>| {
                // Require two integer arguments; wrong arity or type surfaces as
                // `R::Error` and is turned into a catchable JS exception.
                let a: i32 = args.get(scope, 0)?;
                let b: i32 = args.get(scope, 1)?;
                Ok(scope.integer(a + b))
            },
        )?;

        let global = scope.global();
        global.set(scope, "addHost", add);

        let out = scope.eval("addHost(20, 22);")?;
        let sum = out.as_i32(scope).ok_or_else(|| {
            QuickJsError::from(rjsi::HostError::type_error(rjsi::E_TYPE, "expected int result"))
        })?;
        assert_eq!(sum, 42);
        println!("addHost(20, 22) => {sum}");

        Ok(())
    })
}
