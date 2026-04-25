//! Ergonomics extensions on top of a [`rjsi_quickjs::QuickJsRuntimeContext`].

use rjsi_core::convert::{FromJs, ScopeExt};
use rjsi_core::{JsRuntime, JsScope, Source};
use rjsi_quickjs::QuickJsRuntimeContext;

#[test]
fn extension_get_set_and_convert() {
    let runtime = QuickJsRuntimeContext::new();
    runtime
        .with_scope(|scope| {
            let object = scope.eval(Source::from_bytes("({})"))?;
            scope.set(&object, "answer", 42.0).unwrap();
            let value = scope.get(&object, "answer").unwrap().unwrap();
            assert_eq!(f64::from_js(scope, &value)?, 42.0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn extension_call_uses_small_args() {
    let runtime = QuickJsRuntimeContext::new();
    runtime
        .with_scope(|scope| {
            let function = scope.eval(Source::from_bytes("(x => x + 1)"))?;
            let value = scope.call(&function, None, (41.0,)).unwrap();
            assert_eq!(f64::from_js(scope, &value)?, 42.0);
            Ok(())
        })
        .unwrap();
}
