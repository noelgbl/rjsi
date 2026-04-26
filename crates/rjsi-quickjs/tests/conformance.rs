use rjsi_conformance as conformance;
use rjsi_quickjs::{QuickJsRuntime, QuickJsRuntimeContext};

fn rt() -> QuickJsRuntimeContext {
    QuickJsRuntimeContext::new()
}

#[test]
fn eval_runs_in_scope() {
    conformance::eval_runs::<QuickJsRuntime>(&rt());
}

#[test]
fn explicit_global_restores() {
    conformance::explicit_global_restores::<QuickJsRuntime>(&rt());
}

#[test]
fn property_get_set() {
    conformance::static_property_get_set::<QuickJsRuntime>(&rt());
}

#[test]
fn nested_scopes() {
    conformance::nested_scopes::<QuickJsRuntime>(&rt());
}

#[test]
fn constructors_and_host_function_work() {
    conformance::constructors_and_host::<QuickJsRuntime>(&rt());
}

#[test]
fn primitives_roundtrip() {
    conformance::primitives_roundtrip::<QuickJsRuntime>(&rt());
}

#[test]
fn array_index_get_set() {
    conformance::array_index_get_set::<QuickJsRuntime>(&rt());
}

#[test]
fn run_full_conformance_suite() {
    conformance::run_all::<QuickJsRuntime>(&rt());
}

#[test]
fn console_module_smoke() {
    rjsi_console::smoke_install_and_log::<QuickJsRuntime>(&rt()).unwrap();
}
