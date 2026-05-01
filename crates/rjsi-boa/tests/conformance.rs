use rjsi_boa::BoaRuntime;
use rjsi_conformance as conformance;

fn rt() -> BoaRuntime {
    BoaRuntime::new()
}

#[test]
fn eval_runs_in_scope() {
    conformance::eval_runs(&mut rt());
}

#[test]
fn explicit_global_restores() {
    conformance::explicit_global_restores(&mut rt());
}

#[test]
fn property_get_set() {
    conformance::static_property_get_set(&mut rt());
}

#[test]
fn nested_scopes() {
    conformance::nested_scopes(&mut rt());
}

#[test]
fn constructors_and_host_function_work() {
    conformance::constructors_and_host(&mut rt());
}

#[test]
fn primitives_roundtrip() {
    conformance::primitives_roundtrip(&mut rt());
}

#[test]
fn array_index_get_set() {
    conformance::array_index_get_set(&mut rt());
}

#[test]
fn run_full_conformance_suite() {
    conformance::run_all(&mut rt());
}
