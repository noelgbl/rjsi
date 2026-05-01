use rjsi_core::{Engine, Runtime, Value};

fn expect_js<T, E>(r: Result<T, E>, msg: &'static str) -> T {
    r.unwrap_or_else(|_| panic!("{msg}"))
}

pub fn eval_runs<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let value = cx.eval("21 + 21")?;
            assert!(value.is_number());
            Ok(())
        })
        .unwrap();
}

pub fn explicit_global_restores<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let global = cx.globals();
            let value = cx.number(42.0);
            global.set(cx, "answer", value)?;
            Ok(())
        })
        .unwrap();

    runtime
        .with(|cx| {
            let global = cx.globals();
            let restored = global.get(cx, "answer")?;
            let n = expect_js(restored.to_f64(cx), "global restore");
            assert_eq!(n, 42.0);
            Ok(())
        })
        .unwrap();
}

pub fn static_property_get_set<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let object = expect_js(cx.new_object(), "new object");
            let value = cx.number(42.0);
            object.set(cx, "answer", value)?;
            let restored = object.get(cx, "answer")?;
            let n = expect_js(restored.to_f64(cx), "object get");
            assert_eq!(n, 42.0);
            Ok(())
        })
        .unwrap();
}

pub fn nested_scopes<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let outer_value = cx.number(1.0);
            {
                let inner_value = cx.number(2.0);
                let n = expect_js(inner_value.to_f64(cx), "inner value");
                assert_eq!(n, 2.0);
            }
            let n = expect_js(outer_value.to_f64(cx), "outer value");
            assert_eq!(n, 1.0);
            Ok(())
        })
        .unwrap();
}

pub fn constructors_and_host<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let object = expect_js(cx.new_object(), "new object");
            let object_value: Value<'_, E> = object.into_value();
            assert!(object_value.is_object());

            let array_value = cx.eval("new Array(2)")?;
            assert!(array_value.is_array());

            let buffer_value = cx.eval("new ArrayBuffer(3)")?;
            assert!(buffer_value.is_object());

            let fn_value = cx.eval("(n) => n + 1")?;
            let function = expect_js(fn_value.try_as_function(), "conformance: function");
            let arg = cx.number(41.0);
            let this = cx.undefined();
            let result = expect_js(function.call(cx, this, &[arg]), "conformance: call");
            let n = expect_js(result.to_f64(cx), "conformance: call result");
            assert_eq!(n, 42.0);
            Ok(())
        })
        .unwrap();
}

pub fn primitives_roundtrip<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let n = cx.number(-1.5);
            let n = expect_js(n.to_f64(cx), "number roundtrip");
            assert_eq!(n, -1.5);

            let s = expect_js(cx.string("conformance"), "string create");
            let s = expect_js(s.to_string(cx), "string utf8");
            assert_eq!(s, "conformance");

            let b = cx.boolean(false);
            assert_eq!(b.to_bool(), Some(false));
            Ok(())
        })
        .unwrap();
}

pub fn array_index_get_set<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime
        .with(|cx| {
            let array_value = cx.eval("new Array(3)")?;
            assert!(array_value.is_array());
            let array_obj = expect_js(array_value.try_as_object(), "array object");
            let n = cx.number(99.0);
            array_obj.set(cx, 1u32, n)?;
            let got = array_obj.get(cx, 1u32)?;
            let n = expect_js(got.to_f64(cx), "array index");
            assert_eq!(n, 99.0);
            Ok(())
        })
        .unwrap();
}

pub fn run_all<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    eval_runs::<E, R>(runtime);
    explicit_global_restores::<E, R>(runtime);
    static_property_get_set::<E, R>(runtime);
    constructors_and_host::<E, R>(runtime);
    primitives_roundtrip::<E, R>(runtime);
    array_index_get_set::<E, R>(runtime);
}
