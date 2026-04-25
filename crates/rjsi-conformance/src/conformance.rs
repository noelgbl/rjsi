use std::marker::PhantomData;

use rjsi_core::{
    Global, HostFunction, JsEngine, JsRuntime, JsScope, JsValueType, ParamsAccessor, Source,
};

fn expect_js<T, E>(r: Result<T, E>, msg: &'static str) -> T {
    r.unwrap_or_else(|_| panic!("{msg}"))
}

/// Host function used by [`constructors_and_host`]: `addOne(n) => n + 1`.
pub struct AddOne<E: JsEngine>(pub PhantomData<fn() -> E>);

impl<E: JsEngine> HostFunction<E> for AddOne<E> {
    fn call<'a, 'js>(
        &mut self,
        params: &mut ParamsAccessor<'a, 'js, E>,
    ) -> rjsi_core::JsResult<E::Value<'js>>
    where
        'js: 'a,
    {
        let value = params.next_arg().unwrap();
        let number = params.scope().to_number(&value).unwrap();
        Ok(params.scope().number(number + 1.0))
    }
}

/// Eval returns a value with a known type.
pub fn eval_runs<R: JsRuntime>(runtime: &R) {
    runtime
        .with_scope(|scope| {
            let value = scope.eval(Source::from_bytes("21 + 21"))?;
            assert_ne!(scope.value_type(&value), JsValueType::Unknown);
            Ok(())
        })
        .unwrap();
}

/// [`Global`](crate::Global) values survive to a later `with_scope` and match.
pub fn explicit_global_restores<R: JsRuntime>(runtime: &R) {
    let global = runtime
        .with_scope(|scope| {
            let value = scope.number(42.0);
            Ok(Global::<<R as JsRuntime>::Engine>::new(scope, &value))
        })
        .unwrap();
    runtime
        .with_scope(|scope| {
            let restored = global.get(scope);
            assert_eq!(scope.value_type(&restored), JsValueType::Number);
            assert_eq!(scope.to_number(&restored), Some(42.0));
            Ok(())
        })
        .unwrap();
}

/// String-key property get / set on a plain object.
pub fn static_property_get_set<R: JsRuntime>(runtime: &R) {
    runtime
        .with_scope(|scope| {
            let object = scope.eval(Source::from_bytes("({})"))?;
            let key = scope.static_property_key("answer");
            let value = scope.number(42.0);
            expect_js(
                scope.set_property(&object, &key, &value),
                "conformance: set_property",
            );
            let restored = expect_js(
                scope.get_property(&object, &key),
                "conformance: get_property",
            )
            .unwrap();
            assert_eq!(scope.to_number(&restored), Some(42.0));
            Ok(())
        })
        .unwrap();
}

/// Nested `with_scope` on the same runtime: inner scope does not clobber the outer.
pub fn nested_scopes<R: JsRuntime>(runtime: &R) {
    runtime
        .with_scope(|outer| {
            let outer_value = outer.number(1.0);
            runtime
                .with_scope(|inner| {
                    let inner_value = inner.number(2.0);
                    assert_eq!(inner.to_number(&inner_value), Some(2.0));
                    Ok(())
                })
                .unwrap();
            assert_eq!(outer.to_number(&outer_value), Some(1.0));
            Ok(())
        })
        .unwrap();
}

/// Object, array, `ArrayBuffer`, and a registered host function callable from script.
pub fn constructors_and_host<R: JsRuntime>(runtime: &R) {
    runtime
        .with_scope(|scope| {
            let object = scope.object();
            assert_eq!(scope.value_type(&object), JsValueType::Object);

            let array = scope.array(2);
            assert_eq!(scope.value_type(&array), JsValueType::Array);

            let buffer = scope.array_buffer_copy(&[1, 2, 3]);
            assert_ne!(scope.value_type(&buffer), JsValueType::Unknown);

            let function = expect_js(
                scope.host_function("addOne", AddOne::<<R as JsRuntime>::Engine>(PhantomData)),
                "conformance: host_function",
            );
            let arg = scope.number(41.0);
            let result = expect_js(
                scope.call_function(&function, None, &[arg]),
                "conformance: call_function",
            );
            assert_eq!(scope.to_number(&result), Some(42.0));
            Ok(())
        })
        .unwrap();
}

/// Number / string / boolean values round-trip through the scope.
pub fn primitives_roundtrip<R: JsRuntime>(runtime: &R) {
    runtime
        .with_scope(|scope| {
            let n = scope.number(-1.5);
            assert_eq!(scope.to_number(&n), Some(-1.5));
            let s = scope.string("conformance");
            assert_eq!(scope.to_string(&s).as_deref(), Some("conformance"));
            let b = scope.boolean(false);
            assert_eq!(scope.to_boolean(&b), Some(false));
            Ok(())
        })
        .unwrap();
}

/// Array element set / get by index.
pub fn array_index_get_set<R: JsRuntime>(runtime: &R) {
    runtime
        .with_scope(|scope| {
            let a = scope.array(3);
            let n = scope.number(99.0);
            expect_js(scope.set_index(&a, 1, &n), "conformance: set_index");
            let got = expect_js(
                scope.get_index(&a, 1),
                "conformance: get_index",
            )
            .unwrap();
            assert_eq!(scope.to_number(&got), Some(99.0));
            Ok(())
        })
        .unwrap();
}

/// Runs the portable part of this module (everything except [`nested_scopes`], which some
/// backends cannot support when `with_scope` is non-reentrant).
pub fn run_all<R: JsRuntime>(runtime: &R) {
    eval_runs(runtime);
    explicit_global_restores(runtime);
    static_property_get_set(runtime);
    constructors_and_host(runtime);
    primitives_roundtrip(runtime);
    array_index_get_set(runtime);
}
