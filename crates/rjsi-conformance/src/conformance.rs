use rjsi_core::{Args, CallbackCx, Engine, JsError, JsResult, PreparedKey, Runtime, Value};

fn expect_js<T, E>(r: Result<T, E>, msg: &'static str) -> T {
    r.unwrap_or_else(|_| panic!("{msg}"))
}

fn conformance_sum_args<'cx, 'rt, E: Engine>(
    cb: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<'rt, E, Value<'rt, E>> {
    let cx = cb.cx();
    let mut acc = 0.0f64;
    for i in 0..args.len() {
        let v = args
            .get(i)
            .ok_or_else(|| JsError::type_err("missing arg"))?;
        acc += v.to_f64(cx)?;
    }
    Ok(cx.number(acc))
}

fn conformance_greet<'cx, 'rt, E: Engine>(
    cb: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    _args: Args<'rt, E>,
) -> JsResult<'rt, E, Value<'rt, E>> {
    cb.cx().string("hello")
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
        assert_eq!(b.to_bool(), Some(false));
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
        assert_eq!(t.to_bool(), Some(true));
        let back = cx.eval("true").unwrap();
        assert_eq!(back.to_bool(), Some(true));
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
        let sum = expect_js(cx.function("sumArgs", conformance_sum_args), "host fn");
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
        let greet = expect_js(cx.function("greet", conformance_greet), "host greet");
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
            matches!(err, JsError::Exception(_)),
            "expected JS exception for syntax error"
        );
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
    let key: &'static PreparedKey<E> = Box::leak(Box::new(PreparedKey::new("preparedAnswer")));

    runtime.with_scope(move |cx| {
        let global = cx.globals();
        let value = cx.number(42.0);
        global.set(cx, key, value).unwrap();
    });

    runtime.with_scope(move |cx| {
        let global = cx.globals();
        assert!(expect_js(global.has(cx, key), "prepared key has after set"));

        let got = global.get(cx, key).unwrap();
        let n = expect_js(got.to_f64(cx), "prepared key get");
        assert_eq!(n, 42.0);

        let deleted = expect_js(global.delete(cx, key), "prepared key delete");
        assert!(deleted);
        assert!(!expect_js(
            global.has(cx, key),
            "prepared key gone after delete"
        ));
    });
}

pub fn prepared_key_works_inside_host_callback<E, R>(runtime: &mut R)
where
    E: Engine,
    R: Runtime<E>,
{
    struct InstallPrepared<E: Engine> {
        key: &'static PreparedKey<E>,
    }

    impl<E: Engine> rjsi_core::RawHostFn<E> for InstallPrepared<E> {
        fn call<'cx, 'rt>(
            &mut self,
            cb: &mut CallbackCx<'cx, 'rt, E>,
            _this: Value<'rt, E>,
            _args: Args<'rt, E>,
        ) -> JsResult<'rt, E, Value<'rt, E>> {
            let cx = cb.cx();
            let global = cx.globals();
            let value = cx.number(7.0);
            global.set(cx, self.key, value)?;
            Ok(cx.undefined())
        }
    }

    let key: &'static PreparedKey<E> = Box::leak(Box::new(PreparedKey::new("preparedFromHost")));

    runtime.with_scope(move |cx| {
        let install = expect_js(
            cx.function("installPrepared", InstallPrepared { key }),
            "prepared host function",
        );

        let global = cx.globals();
        global
            .set(cx, "installPrepared", install.into_value())
            .unwrap();
        cx.eval("installPrepared()").unwrap();

        let got = global.get(cx, key).unwrap();
        let n = expect_js(got.to_f64(cx), "prepared key host callback");
        assert_eq!(n, 7.0);
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
    json_parse_roundtrip(runtime);
    array_spread_and_length(runtime);
    template_literal_basic(runtime);
    optional_chaining_and_nullish_coalescing(runtime);
    number_to_string_coercion(runtime);
    prepared_key_roundtrip_across_scopes(runtime);
    prepared_key_works_inside_host_callback(runtime);
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
