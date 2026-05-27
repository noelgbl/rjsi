use rjsi_core::capabilities::{Buffers, Float32Array, TypedArrayKind, Uint8Array};
use rjsi_core::{
    Args, CatchResultExt, CaughtError, Context, ContextBufferExt, ContextNativeStateExt, Engine, Error, NativeState, NativeStateSupport, PersistentValue, PreparedKey, Result, Runtime, Value
};

fn expect_js<T, E>(r: std::result::Result<T, E>, msg: &'static str) -> T {
    r.unwrap_or_else(|_| panic!("{msg}"))
}

fn conformance_sum_args<'js, E: Engine>(
    cx: &mut Context<'js, E>,
    _this: Value<'js, E>,
    args: Args<'js, E>,
) -> Result<Value<'js, E>> {
    let mut acc = 0.0f64;
    for i in 0..args.len() {
        let v = args.get(i).ok_or_else(|| Error::type_err("missing arg"))?;
        acc += v.to_f64(cx)?;
    }
    Ok(cx.number(acc))
}

fn conformance_greet<'js, E: Engine>(
    cx: &mut Context<'js, E>,
    _this: Value<'js, E>,
    _args: Args<'js, E>,
) -> Result<Value<'js, E>> {
    cx.string("hello")
}

pub fn persistent_survives_across_scopes<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    let persisted = runtime.with_scope(|cx| {
        let obj = expect_js(cx.eval("({ tag: 42 })"), "eval object");
        PersistentValue::persist(cx, obj)
    });

    runtime.with_scope(|cx| {
        let val = expect_js(persisted.restore(cx), "restore persistent");
        let obj = expect_js(val.try_as_object(), "as object");
        let tag = expect_js(obj.get_typed::<f64>(cx, "tag"), "tag property");
        assert_eq!(tag, 42.0);
    });
}

pub fn eval_runs<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let value = cx.eval("21 + 21").unwrap();
        assert!(value.is_number());
    });
}

pub fn explicit_global_restores<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let global = cx.globals();
        let value = cx.number(42.0);
        global.set(cx, "answer", value).unwrap();
    });

    runtime.with_scope(|cx| {
        let global = cx.globals();
        let restored = global.get(cx, "answer").unwrap();
        let n = expect_js(restored.to_f64(cx), "global restore");
        assert_eq!(n, 42.0);
    });
}

pub fn static_property_get_set<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let object = expect_js(cx.new_object(), "new object");
        let value = cx.number(42.0);
        object.set(cx, "answer", value).unwrap();
        let restored = object.get(cx, "answer").unwrap();
        let n = expect_js(restored.to_f64(cx), "object get");
        assert_eq!(n, 42.0);
    });
}

pub fn nested_scopes<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let outer_value = cx.number(1.0);
        {
            let inner_value = cx.number(2.0);
            let n = expect_js(inner_value.to_f64(cx), "inner value");
            assert_eq!(n, 2.0);
        }
        let n = expect_js(outer_value.to_f64(cx), "outer value");
        assert_eq!(n, 1.0);
    });
}

pub fn constructors_and_host<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let object = expect_js(cx.new_object(), "new object");
        let object_value: Value<'_, E> = object.into_value();
        assert!(object_value.is_object());

        let array_value = cx.eval("new Array(2)").unwrap();
        assert!(array_value.is_array());

        let buffer_value = cx.eval("new ArrayBuffer(3)").unwrap();
        assert!(buffer_value.is_object());

        let fn_value = cx.eval("(n) => n + 1").unwrap();
        let function = expect_js(fn_value.try_as_function(), "conformance: function");
        let arg = cx.number(41.0);
        let this = cx.undefined();
        let result = expect_js(function.call(cx, this, &[arg]), "conformance: call");
        let n = expect_js(result.to_f64(cx), "conformance: call result");
        assert_eq!(n, 42.0);
    });
}

pub fn primitives_roundtrip<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let n = cx.number(-1.5);
        let n = expect_js(n.to_f64(cx), "number roundtrip");
        assert_eq!(n, -1.5);

        let s = expect_js(cx.string("conformance"), "string create");
        let s = expect_js(s.to_string(cx), "string utf8");
        assert_eq!(s, "conformance");

        let b = cx.boolean(false);
        assert_eq!(b.as_bool(), Some(false));
    });
}

pub fn array_index_get_set<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let array_value = cx.eval("new Array(3)").unwrap();
        assert!(array_value.is_array());
        let array_obj = expect_js(array_value.try_as_object(), "array object");
        let n = cx.number(99.0);
        array_obj.set(cx, 1u32, n).unwrap();
        let got = array_obj.get(cx, 1u32).unwrap();
        let n = expect_js(got.to_f64(cx), "array index");
        assert_eq!(n, 99.0);
    });
}

pub fn null_undefined_discriminators<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let u = cx.undefined();
        assert!(u.is_undefined());
        assert!(!u.is_null());
        assert!(u.is_nullish());

        let n = cx.null();
        assert!(n.is_null());
        assert!(!n.is_undefined());
        assert!(n.is_nullish());
    });
}

pub fn boolean_true_roundtrip<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let t = cx.boolean(true);
        assert_eq!(t.as_bool(), Some(true));
        let back = cx.eval("true").unwrap();
        assert_eq!(back.as_bool(), Some(true));
    });
}

pub fn integer_i32_extremes<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        for v in [0i32, -1, i32::MAX, i32::MIN] {
            let js = cx.integer(v);
            let n = expect_js(js.to_f64(cx), "i32 to f64");
            assert_eq!(n, f64::from(v));
        }
    });
}

pub fn eval_with_filename_basic<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx
            .eval_with_filename("7 * 6", "conformance-suite.js")
            .unwrap();
        let n = expect_js(v.to_f64(cx), "eval with filename");
        assert_eq!(n, 42.0);
    });
}

pub fn object_has_delete_own_property<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let o = expect_js(cx.new_object(), "object");
        assert!(!expect_js(o.has(cx, "k"), "missing before set"));

        let one = cx.number(1.0);
        o.set(cx, "k", one).unwrap();
        assert!(expect_js(o.has(cx, "k"), "present after set"));

        let deleted = expect_js(o.delete(cx, "k"), "delete");
        assert!(deleted);
        assert!(!expect_js(o.has(cx, "k"), "gone after delete"));

        let deleted_again = expect_js(o.delete(cx, "k"), "delete missing");
        assert!(deleted_again);
    });
}

pub fn object_get_missing_is_undefined<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let o = expect_js(cx.new_object(), "object");
        let v = o.get(cx, "absent").unwrap();
        assert!(v.is_undefined());
    });
}

pub fn unicode_string_property_key<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let o = expect_js(cx.new_object(), "object");
        let key = "café";
        let ok = expect_js(cx.string("ok"), "ok string");
        o.set(cx, key, ok).unwrap();
        let got = expect_js(o.get(cx, key).unwrap().to_string(cx), "utf8 get");
        assert_eq!(got, "ok");
    });
}

pub fn host_function_sums_arguments<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let sum = expect_js(cx.raw_function("sumArgs", conformance_sum_args), "host fn");
        let global = cx.globals();
        global.set(cx, "sumArgs", sum.into_value()).unwrap();
        let v = cx.eval("sumArgs(1, 2, 3, 4)").unwrap();
        assert_eq!(expect_js(v.to_f64(cx), "sum"), 10.0);
    });
}

pub fn host_function_returns_string<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let greet = expect_js(cx.raw_function("greet", conformance_greet), "host greet");
        cx.globals().set(cx, "greet", greet.into_value()).unwrap();
        let v = cx.eval("greet()").unwrap();
        assert_eq!(expect_js(v.to_string(cx), "greet result"), "hello");
    });
}

pub fn js_function_call_no_args<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let f = expect_js(cx.eval("() => 123").unwrap().try_as_function(), "fn");
        let out = expect_js(f.call_no_args(cx), "call no args");
        assert_eq!(expect_js(out.to_f64(cx), "n"), 123.0);
    });
}

pub fn strict_mode_this_undefined_when_calling_with_undefined<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx
            .eval(
                "'use strict'; \
                 (function () { return this; }).call(undefined)",
            )
            .unwrap();
        assert!(v.is_undefined());
    });
}

pub fn eval_syntax_error_surfaces<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let res = cx.eval("@@@not_valid_js@@@");
        assert!(res.is_err(), "expected syntax error");
        let err = res.err().expect("err");
        assert!(
            matches!(err, Error::Exception),
            "expected JS exception for syntax error"
        );
    });
}

pub fn exception_value_is_accessible<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let res = cx.eval("throw new Error('rjsi-test')");
        assert!(matches!(res, Err(Error::Exception)));
        let exc = cx.catch_exception();
        assert!(
            exc.is_some(),
            "engine must expose the thrown value via catch_exception"
        );
        let exc = exc.unwrap();
        assert!(exc.is_object(), "thrown Error must be an object");
    });
}

pub fn caught_error_object_classified_as_exception<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let caught = cx.eval("throw new Error('boom')").catch(cx);
        match caught {
            Err(CaughtError::Exception(ex)) => {
                assert_eq!(ex.message(cx).as_deref(), Some("boom"));
                assert_eq!(ex.name(cx).as_deref(), Some("Error"));
            }
            Err(other) => panic!("expected CaughtError::Exception, got {:?}", other),
            Ok(_) => panic!("expected Err"),
        }
    });
}

pub fn caught_error_primitive_classified_as_value<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let caught = cx.eval("throw 42").catch(cx);
        match caught {
            Err(CaughtError::Value(v)) => {
                let n = expect_js(v.to_f64(cx), "thrown number");
                assert_eq!(n, 42.0);
            }
            Err(other) => panic!("expected CaughtError::Value, got {:?}", other),
            Ok(_) => panic!("expected Err"),
        }
    });
}

fn throw_from_rust<'js, E: Engine>(
    cx: &mut Context<'js, E>,
    _this: Value<'js, E>,
    _args: Args<'js, E>,
) -> Result<Value<'js, E>> {
    let msg = cx.string("from rust")?;
    Err(cx.throw(msg))
}

pub fn throw_result_ext_round_trip<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let thrower = cx
            .raw_function("__rjsi_thrower", throw_from_rust::<E>)
            .expect("raw_function");
        cx.globals()
            .set(cx, "__thrower", thrower.into_value())
            .expect("set global");
        let v = cx
            .eval("(function(){ try { __thrower() } catch (e) { return e } return 'nope' })()")
            .expect("catch in JS");
        let s = expect_js(v.to_string(cx), "thrown string");
        assert_eq!(s, "from rust");
    });
}

pub fn json_parse_roundtrip<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx.eval("JSON.parse('{\"x\":9}').x").unwrap();
        assert_eq!(expect_js(v.to_f64(cx), "json x"), 9.0);
    });
}

pub fn array_spread_and_length<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx.eval("[...[1, 2], 3].length").unwrap();
        assert_eq!(expect_js(v.to_f64(cx), "len"), 3.0);
    });
}

pub fn template_literal_basic<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx.eval("`${1}${2}`").unwrap();
        assert_eq!(expect_js(v.to_string(cx), "tpl"), "12");
    });
}

pub fn optional_chaining_and_nullish_coalescing<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx.eval("null?.x ?? 5").unwrap();
        assert_eq!(expect_js(v.to_f64(cx), "??"), 5.0);
    });
}

pub fn number_to_string_coercion<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let v = cx.eval("String(3.25)").unwrap();
        assert_eq!(expect_js(v.to_string(cx), "str"), "3.25");
    });
}

pub fn prepared_key_roundtrip_across_scopes<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    let key = PreparedKey::new("preparedAnswer");

    runtime.with_scope({
        let key = key.clone();
        move |cx| {
            let global = cx.globals();
            let value = cx.number(42.0);
            global.set(cx, &key, value).unwrap();
        }
    });

    runtime.with_scope({
        let key = key.clone();
        move |cx| {
            let global = cx.globals();
            assert!(expect_js(
                global.has(cx, &key),
                "prepared key has after set"
            ));

            let got = global.get(cx, &key).unwrap();
            let n = expect_js(got.to_f64(cx), "prepared key get");
            assert_eq!(n, 42.0);

            let deleted = expect_js(global.delete(cx, &key), "prepared key delete");
            assert!(deleted);
            assert!(!expect_js(
                global.has(cx, &key),
                "prepared key gone after delete"
            ));
        }
    });
}

pub fn prepared_key_works_inside_host_callback<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    struct InstallPrepared<E: Engine> {
        key: PreparedKey<E>,
    }

    impl<E: Engine> rjsi_core::RawHostFn<E> for InstallPrepared<E> {
        fn call<'js>(
            &mut self,
            cx: &mut Context<'js, E>,
            _this: Value<'js, E>,
            _args: Args<'js, E>,
        ) -> Result<Value<'js, E>> {
            let global = cx.globals();
            let value = cx.number(7.0);
            global.set(cx, &self.key, value)?;
            Ok(cx.undefined())
        }
    }

    let key = PreparedKey::new("preparedFromHost");

    runtime.with_scope(move |cx| {
        let install = expect_js(
            cx.raw_function("installPrepared", InstallPrepared { key: key.clone() }),
            "prepared host function",
        );

        let global = cx.globals();
        global
            .set(cx, "installPrepared", install.into_value())
            .unwrap();
        cx.eval("installPrepared()").unwrap();

        let got = global.get(cx, &key).unwrap();
        let n = expect_js(got.to_f64(cx), "prepared key host callback");
        assert_eq!(n, 7.0);
    });
}

struct NativeStateTestPayload(i32);

impl NativeState for NativeStateTestPayload {}

pub fn native_state_roundtrip<E, R>(runtime: &mut R)
where
    E: Engine + NativeStateSupport,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let mut obj = cx
            .with_state(NativeStateTestPayload(7))
            .expect("create_with_state");
        assert_eq!(
            obj.native_state::<NativeStateTestPayload>(cx)
                .expect("native_state")
                .0,
            7
        );
        obj.native_state_mut::<NativeStateTestPayload>(cx)
            .expect("native_state_mut")
            .0 = 42;
        assert_eq!(
            obj.native_state::<NativeStateTestPayload>(cx)
                .expect("native_state after mut")
                .0,
            42
        );
    });
}

/// Native payload survives [`PersistentValue`] round-trip across separate
/// [`Runtime::with_scope`] calls.
pub fn native_state_persistent_across_scopes<E, R>(runtime: &mut R)
where
    E: Engine + NativeStateSupport,
    R: Runtime<E>,
{
    let persisted = runtime.with_scope(|cx| {
        let obj = cx
            .with_state(NativeStateTestPayload(99))
            .expect("with_state");
        PersistentValue::persist(cx, obj.into_value())
    });

    runtime.with_scope(|cx| {
        let val = expect_js(persisted.restore(cx), "restore persistent");
        let mut obj = expect_js(val.try_as_object(), "as object");
        assert_eq!(
            obj.native_state::<NativeStateTestPayload>(cx)
                .expect("native_state after restore")
                .0,
            99
        );
        obj.native_state_mut::<NativeStateTestPayload>(cx)
            .expect("native_state_mut after restore")
            .0 = 100;
        assert_eq!(
            obj.native_state::<NativeStateTestPayload>(cx)
                .expect("native_state after mut across scopes")
                .0,
            100
        );
    });
}

pub fn typed_function_adds_integers<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let add = expect_js(cx.function("typedAdd", |a: i32, b: i32| a + b), "typed add");
        cx.globals().set(cx, "typedAdd", add.into_value()).unwrap();
        let v = cx.eval("typedAdd(10, 32)").unwrap();
        assert_eq!(expect_js(v.to_f64(cx), "typed add result"), 42.0);
    });
}

pub fn typed_function_cx_builds_string<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let greet = expect_js(
            cx.function(
                "typedGreet",
                |_cx: &mut rjsi_core::Context<E>, name: String| format!("hello {name}"),
            ),
            "typed greet",
        );
        cx.globals()
            .set(cx, "typedGreet", greet.into_value())
            .unwrap();
        let v = cx.eval(r#"typedGreet("world")"#).unwrap();
        assert_eq!(
            expect_js(v.to_string(cx), "typed greet result"),
            "hello world"
        );
    });
}

pub fn run_all<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    eval_runs(runtime);
    explicit_global_restores(runtime);
    static_property_get_set(runtime);
    nested_scopes(runtime);
    constructors_and_host(runtime);
    primitives_roundtrip(runtime);
    array_index_get_set(runtime);
    null_undefined_discriminators(runtime);
    boolean_true_roundtrip(runtime);
    integer_i32_extremes(runtime);
    eval_with_filename_basic(runtime);
    object_has_delete_own_property(runtime);
    object_get_missing_is_undefined(runtime);
    unicode_string_property_key(runtime);
    host_function_sums_arguments(runtime);
    host_function_returns_string(runtime);
    js_function_call_no_args(runtime);
    strict_mode_this_undefined_when_calling_with_undefined(runtime);
    eval_syntax_error_surfaces(runtime);
    exception_value_is_accessible(runtime);
    caught_error_object_classified_as_exception(runtime);
    caught_error_primitive_classified_as_value(runtime);
    throw_result_ext_round_trip(runtime);
    json_parse_roundtrip(runtime);
    array_spread_and_length(runtime);
    template_literal_basic(runtime);
    optional_chaining_and_nullish_coalescing(runtime);
    number_to_string_coercion(runtime);
    prepared_key_roundtrip_across_scopes(runtime);
    prepared_key_works_inside_host_callback(runtime);
    persistent_survives_across_scopes(runtime);
    typed_function_adds_integers(runtime);
    typed_function_cx_builds_string(runtime);
}

pub fn promise_capabilities<E, R>(runtime: &mut R)
where
    E: Engine + rjsi_core::capabilities::Promises + rjsi_core::capabilities::Microtasks,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        use rjsi_core::{ContextMicrotaskExt, ContextPromiseExt};

        let (promise, resolver) = cx.promise_new().unwrap();

        let global = cx.globals();
        global.set(cx, "testPromise", promise.into_value()).unwrap();

        cx.eval("testPromise.then(v => { globalThis.promiseResult = v; })")
            .unwrap();

        let val = cx.number(42.0);
        cx.promise_resolve(resolver, val).unwrap();

        cx.drain_microtasks();

        let result = global.get(cx, "promiseResult").unwrap();
        let n = expect_js(result.to_f64(cx), "promise result");
        assert_eq!(n, 42.0);
    });
}

pub fn js_channel_capabilities<E, R>(runtime: &mut R)
where
    E: Engine + rjsi_core::capabilities::Promises + rjsi_core::capabilities::Microtasks,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let (tx, mut channel) = rjsi_core::channel::JsChannel::<E, f64, String>::new();
        use rjsi_core::ContextMicrotaskExt;

        let (id, promise) = channel.create_promise(cx).unwrap();

        let global = cx.globals();
        global
            .set(cx, "channelPromise", promise.into_value())
            .unwrap();

        cx.eval("channelPromise.then(v => { globalThis.channelResult = v; })")
            .unwrap();

        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            tx_clone.resolve(id, 99.0).unwrap();
        })
        .join()
        .unwrap();

        channel
            .pump(
                cx,
                |cx, val| Ok(cx.number(val)),
                |cx, err| cx.string(&err).map(Into::into),
            )
            .unwrap();

        cx.drain_microtasks();

        let result = global.get(cx, "channelResult").unwrap();
        let n = expect_js(result.to_f64(cx), "channel result");
        assert_eq!(n, 99.0);
    });
}

pub fn tokio_channel_capabilities<E, R>(runtime: &mut R)
where
    E: Engine + rjsi_core::capabilities::Promises + rjsi_core::capabilities::Microtasks,
    R: Runtime<E>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.with_scope(|cx| {
        let (tx, mut channel) = rjsi_core::channel::JsChannel::<E, f64, String>::new();
        use rjsi_core::ContextMicrotaskExt;

        let (id, promise) = channel.create_promise(cx).unwrap();

        let global = cx.globals();
        global
            .set(cx, "tokioPromise", promise.into_value())
            .unwrap();

        cx.eval("tokioPromise.then(v => { globalThis.tokioResult = v; })")
            .unwrap();

        let tx_clone = tx.clone();

        rt.block_on(async {
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                tx_clone.resolve(id, 88.0).unwrap();
            });

            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        });

        channel
            .pump(
                cx,
                |cx, val| Ok(cx.number(val)),
                |cx, err| cx.string(&err).map(Into::into),
            )
            .unwrap();

        cx.drain_microtasks();

        let result = global.get(cx, "tokioResult").unwrap();
        let n = expect_js(result.to_f64(cx), "tokio result");
        assert_eq!(n, 88.0);
    });
}

pub fn array_buffer_alloc_and_inspect<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let buf = expect_js(cx.array_buffer_alloc(16), "alloc 16-byte buffer");
        assert_eq!(expect_js(buf.byte_length(cx), "byte_length"), 16);
        let bytes = expect_js(buf.to_vec(cx), "to_vec");
        assert_eq!(bytes, vec![0u8; 16]);
    });
}

pub fn array_buffer_adopt_vec_visible_to_js<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let buf = expect_js(
            cx.array_buffer_from_vec(vec![10u8, 20, 30, 40]),
            "adopt Vec<u8> as ArrayBuffer",
        );
        let buf_value = buf.into_value();
        let globals = cx.globals();
        expect_js(globals.set(cx, "buf", buf_value), "set globalThis.buf");
        let v = expect_js(cx.eval("new Uint8Array(buf)[2]"), "read buf[2]");
        let n = expect_js(v.to_f64(cx), "as f64");
        assert_eq!(n, 30.0);
    });
}

pub fn uint8_array_from_rust_visible_to_js<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let arr = expect_js(
            cx.uint8_array_from_vec(vec![1u8, 2, 3, 4, 5]),
            "uint8_array_from_vec",
        );
        let val = arr.into_value();
        let globals = cx.globals();
        expect_js(globals.set(cx, "a", val), "set globalThis.a");
        let len = expect_js(cx.eval("a.length"), "a.length");
        assert_eq!(expect_js(len.to_f64(cx), "len f64"), 5.0);
        let v = expect_js(cx.eval("a[3]"), "a[3]");
        assert_eq!(expect_js(v.to_f64(cx), "v f64"), 4.0);
    });
}

pub fn typed_array_from_js_readable_in_rust<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let val = expect_js(
            cx.eval("new Float32Array([1.5, 2.5, 3.5])"),
            "eval Float32Array",
        );
        assert_eq!(
            E::value_typed_array_kind(val.as_raw()),
            Some(TypedArrayKind::Float32),
            "kind detection"
        );
        let obj = expect_js(val.try_as_object(), "as object");
        let arr = Float32Array::<E>::new(obj);
        let info = expect_js(arr.info(cx), "info");
        assert_eq!(info.kind, TypedArrayKind::Float32);
        assert_eq!(info.byte_offset, 0);
        assert_eq!(info.byte_length, 12);
        assert_eq!(info.length, 3);
        let v = expect_js(arr.to_vec(cx), "to_vec");
        assert_eq!(v, vec![1.5f32, 2.5, 3.5]);
    });
}

pub fn vec_u8_round_trip<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        use rjsi_core::ToJs;
        let original = vec![7u8, 11, 13, 17, 19];
        let val = expect_js(original.clone().to_js(cx), "to_js Vec<u8>");
        let globals = cx.globals();
        expect_js(globals.set(cx, "v", val), "set globalThis.v");
        let mirror = expect_js(cx.eval("v"), "fetch v back");
        let round_trip: Vec<u8> = expect_js(
            <Vec<u8> as rjsi_core::FromJs<E>>::from_js(cx, mirror),
            "FromJs Vec<u8>",
        );
        assert_eq!(round_trip, original);
    });
}

pub fn typed_array_byte_offset_view<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let val = expect_js(
            cx.eval("(() => { const b = new ArrayBuffer(8); const u = new Uint8Array(b); for (let i=0;i<8;i++) u[i]=i+1; return new Uint16Array(b, 2, 2); })()"),
            "eval offset view",
        );
        assert_eq!(
            E::value_typed_array_kind(val.as_raw()),
            Some(TypedArrayKind::Uint16),
        );
        let obj = expect_js(val.try_as_object(), "as object");
        let info = expect_js(E::typed_array_info(cx, obj.as_raw()), "info");
        assert_eq!(info.kind, TypedArrayKind::Uint16);
        assert_eq!(info.byte_offset, 2);
        assert_eq!(info.byte_length, 4);
        assert_eq!(info.length, 2);

        let mut bytes = vec![0u8; info.byte_length];
        expect_js(
            E::typed_array_copy_to(cx, obj.as_raw(), &mut bytes),
            "typed_array_copy_to",
        );
        assert_eq!(bytes, vec![3u8, 4, 5, 6]);
    });
}

pub fn append_to_reuses_allocation<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let arr1 = expect_js(cx.uint8_array_from_vec(vec![1u8, 2, 3]), "first array");
        let mut pool: Vec<u8> = Vec::with_capacity(16);
        expect_js(arr1.append_to(cx, &mut pool), "append first");
        assert_eq!(pool, vec![1u8, 2, 3]);
        pool.clear();
        let arr2 = expect_js(
            cx.uint8_array_from_vec(vec![9u8, 8, 7, 6, 5]),
            "second array",
        );
        expect_js(arr2.append_to(cx, &mut pool), "append second");
        assert_eq!(pool, vec![9u8, 8, 7, 6, 5]);
    });
}

pub fn from_js_uint8_array_wrapper<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    runtime.with_scope(|cx| {
        let val = expect_js(cx.eval("new Uint8Array([42, 43, 44])"), "eval Uint8Array");
        let arr: Uint8Array<E> = expect_js(
            <Uint8Array<E> as rjsi_core::FromJs<E>>::from_js(cx, val),
            "FromJs Uint8Array",
        );
        let v = expect_js(arr.to_vec(cx), "to_vec");
        assert_eq!(v, vec![42u8, 43, 44]);
    });
}

pub fn buffer_capabilities_runs_all<E, R>(runtime: &mut R)
where
    E: Engine + Buffers,
    R: Runtime<E>,
{
    array_buffer_alloc_and_inspect(runtime);
    array_buffer_adopt_vec_visible_to_js(runtime);
    uint8_array_from_rust_visible_to_js(runtime);
    typed_array_from_js_readable_in_rust(runtime);
    vec_u8_round_trip(runtime);
    typed_array_byte_offset_view(runtime);
    append_to_reuses_allocation(runtime);
    from_js_uint8_array_wrapper(runtime);
}
