use rjsi_conformance as conformance;
use rjsi_v8::{V8Runtime, V8RuntimeContext};

fn rt() -> V8RuntimeContext {
    V8RuntimeContext::new()
}

#[test]
fn eval_runs_in_scope() {
    conformance::eval_runs::<V8Runtime>(&rt());
}

#[test]
fn explicit_global_restores() {
    conformance::explicit_global_restores::<V8Runtime>(&rt());
}

#[test]
fn property_get_set() {
    conformance::static_property_get_set::<V8Runtime>(&rt());
}

#[test]
fn nested_scope_reuses_active_isolate() {
    conformance::nested_scopes::<V8Runtime>(&rt());
}

#[test]
fn constructors_and_host_function_work() {
    conformance::constructors_and_host::<V8Runtime>(&rt());
}

#[test]
fn primitives_roundtrip() {
    conformance::primitives_roundtrip::<V8Runtime>(&rt());
}

#[test]
fn array_index_get_set() {
    conformance::array_index_get_set::<V8Runtime>(&rt());
}

#[test]
fn run_full_conformance_suite() {
    conformance::run_all::<V8Runtime>(&rt());
}

#[test]
fn console_module_smoke() {
    rjsi_console::smoke_install_and_log::<V8Runtime>(&rt()).unwrap();
}
