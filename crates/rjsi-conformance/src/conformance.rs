use rjsi_core::{bind, ContextLike, Global, Runtime, ScopeLike, ValueLike};

fn expect_js<T, E>(r: Result<T, E>, msg: &'static str) -> T {
    r.unwrap_or_else(|_| panic!("{msg}"))
}

pub fn eval_runs<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime
        .with_scope(|scope| {
            let value = scope.eval("21 + 21")?;
            assert!(value.is_number());
            Ok(())
        })
        .unwrap();
}

pub fn explicit_global_restores<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    let global = runtime
        .with_scope(|scope| {
            let value = scope.number(42.0);
            Ok(Global::<R>::new(scope, value))
        })
        .unwrap();
    runtime
        .with_scope(|scope| {
            let restored = global.get(scope);
            assert_eq!(restored.as_f64(scope), Some(42.0));
            Ok(())
        })
        .unwrap();
}

pub fn static_property_get_set<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime
        .with_scope(|scope| {
            let object = scope.object();
            let value = scope.number(42.0);
            object.set(scope, "answer", value);
            let restored = object.get(scope, "answer");
            assert_eq!(restored.as_f64(scope), Some(42.0));
            Ok(())
        })
        .unwrap();
}

pub fn nested_scopes<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime
        .with_scope(|outer| {
            let outer_value = outer.number(1.0);
            outer.with_scope(|inner| {
                let inner_value = inner.number(2.0);
                assert_eq!(inner_value.as_f64(inner), Some(2.0));
            });
            assert_eq!(outer_value.as_f64(outer), Some(1.0));
            Ok(())
        })
        .unwrap();
}

pub fn constructors_and_host<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime
        .with_scope(|scope| {
            let object = scope.object();
            assert!(object.is_object());

            let array = scope.array(2);
            assert!(array.is_array());

            let buffer = scope.array_buffer_copy(&[1, 2, 3]);
            assert!(buffer.is_object());

            let function = expect_js(
                scope.function(bind(|_scope, (n,): (f64,)| Ok(n + 1.0))),
                "conformance: function",
            );
            let arg = scope.number(41.0);
            let this = scope.global();
            let result = expect_js(function.call(scope, this, &[arg]), "conformance: call");
            assert_eq!(result.as_f64(scope), Some(42.0));
            Ok(())
        })
        .unwrap();
}

pub fn primitives_roundtrip<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime
        .with_scope(|scope| {
            let n = scope.number(-1.5);
            assert_eq!(n.as_f64(scope), Some(-1.5));
            let s = scope.string("conformance");
            assert_eq!(s.with_str(scope, str::to_owned).as_deref(), Some("conformance"));
            let b = scope.boolean(false);
            assert!(!b.as_bool(scope).unwrap_or(false));
            Ok(())
        })
        .unwrap();
}

pub fn array_index_get_set<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime
        .with_scope(|scope| {
            let a = scope.array(3);
            let n = scope.number(99.0);
            a.set_index(scope, 1, n);
            let got = a.get_index(scope, 1);
            assert_eq!(got.as_f64(scope), Some(99.0));
            Ok(())
        })
        .unwrap();
}

pub fn run_all<R>(runtime: &R::Context)
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    eval_runs::<R>(runtime);
    explicit_global_restores::<R>(runtime);
    static_property_get_set::<R>(runtime);
    constructors_and_host::<R>(runtime);
    primitives_roundtrip::<R>(runtime);
    array_index_get_set::<R>(runtime);
}
