//! Shared [`rjsi_core::conformance`] scenarios for the V8 backend.

use rjsi_conformance::{self as conformance};
use rjsi_v8::V8RuntimeContext;

fn rt() -> V8RuntimeContext {
    V8RuntimeContext::new()
}

#[test]
fn eval_runs_in_scope() {
    conformance::eval_runs(&rt());
}

#[test]
fn explicit_global_restores() {
    conformance::explicit_global_restores(&rt());
}

#[test]
fn static_property_key_get_set() {
    conformance::static_property_get_set(&rt());
}

#[test]
fn nested_scope_reuses_active_isolate() {
    conformance::nested_scopes(&rt());
}

#[test]
fn constructors_and_host_function_work() {
    conformance::constructors_and_host(&rt());
}

#[test]
fn primitives_roundtrip() {
    conformance::primitives_roundtrip(&rt());
}

#[test]
fn array_index_get_set() {
    conformance::array_index_get_set(&rt());
}

#[test]
fn run_full_conformance_suite() {
    conformance::run_all(&rt());
}

#[test]
fn console_module_smoke() {
    rjsi_console::smoke_install_and_log(&rt()).unwrap();
}
