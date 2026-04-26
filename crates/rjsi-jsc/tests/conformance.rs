use rjsi_conformance as conformance;
use rjsi_jsc::{JscRuntime, JscRuntimeContext};

fn rt() -> JscRuntimeContext {
    JscRuntimeContext::new()
}

#[test]
fn eval_runs_in_scope() {
    conformance::eval_runs::<JscRuntime>(&rt());
}

#[test]
fn explicit_global_restores() {
    conformance::explicit_global_restores::<JscRuntime>(&rt());
}

#[test]
fn property_get_set() {
    conformance::static_property_get_set::<JscRuntime>(&rt());
}

#[test]
fn nested_scopes() {
    conformance::nested_scopes::<JscRuntime>(&rt());
}

#[test]
fn constructors_and_host_function_work() {
    conformance::constructors_and_host::<JscRuntime>(&rt());
}

#[test]
fn primitives_roundtrip() {
    conformance::primitives_roundtrip::<JscRuntime>(&rt());
}

#[test]
fn array_index_get_set() {
    conformance::array_index_get_set::<JscRuntime>(&rt());
}

#[test]
fn run_full_conformance_suite() {
    conformance::run_all::<JscRuntime>(&rt());
}

#[test]
fn console_module_smoke() {
    rjsi_console::smoke_install_and_log::<JscRuntime>(&rt()).unwrap();
}
