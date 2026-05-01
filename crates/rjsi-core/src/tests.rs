use crate::mock::{MockContext, MockEngine, MockRawArgs, MockRuntime, MockValue};
use crate::{__cx, Args, Context, JsError, KeyCache, StaticKeySlot};

#[test]
fn args_len_and_get_reflect_raw_argv() {
    let raw = MockRawArgs::from_slice(&[
        MockValue::number(1),
        MockValue::number(2),
        MockValue::number(3),
    ]);
    let args = Args::<MockEngine>::new(raw);
    assert_eq!(args.len(), 3);
    assert_eq!(args.get(0), Some(MockValue::number(1)));
    assert_eq!(args.get(3), None);
    assert!(!args.is_empty());
}

#[test]
fn args_iter_forward_and_rev() {
    let args = Args::<MockEngine>::new(MockRawArgs::from_slice(&[
        MockValue::number(10),
        MockValue::number(20),
        MockValue::number(30),
    ]));
    assert_eq!(
        args.iter().collect::<Vec<_>>(),
        vec![
            MockValue::number(10),
            MockValue::number(20),
            MockValue::number(30),
        ]
    );
    let mut it = args.iter();
    assert_eq!(it.next(), Some(MockValue::number(10)));
    assert_eq!(it.next_back(), Some(MockValue::number(30)));
    assert_eq!(it.next(), Some(MockValue::number(20)));
    assert_eq!(it.next(), None);
}

#[test]
fn args_into_iter_for_ref() {
    let args = Args::<MockEngine>::new(MockRawArgs::from_slice(&[
        MockValue::number(1),
        MockValue::number(2),
    ]));
    let v: Vec<u32> = (&args)
        .into_iter()
        .map(|v| v.tag.saturating_sub(4))
        .collect();
    assert_eq!(v, vec![1, 2]);
}

#[test]
fn args_empty() {
    let args = Args::<MockEngine>::new(MockRawArgs::from_slice(&[]));
    assert_eq!(args.len(), 0);
    assert!(args.is_empty());
}

#[test]
fn key_cache_interns_per_slot() {
    let mut rt = MockRuntime::default();
    let mut cx = MockEngine::detached_cx();
    let before = rt.atoms.len();
    rt.get_or_intern(&mut cx, StaticKeySlot(0)).expect("slot 0");
    let after_first = rt.atoms.len();
    rt.get_or_intern(&mut cx, StaticKeySlot(0))
        .expect("slot 0 again");
    let after_second = rt.atoms.len();
    assert_eq!(after_first, before + 1);
    assert_eq!(after_second, after_first);
}

#[test]
fn cx_with_context_mut_runs_closure() {
    let mut rt = MockRuntime::default();
    let mut cx = Context::<MockEngine>::new(<MockEngine as crate::Engine>::enter(&mut rt));
    let n = cx.with_context_mut(|_ctx: &mut MockContext<'_>| 42u32);
    assert_eq!(n, 42);
}

#[test]
fn cx_internal_context_mut_visible_to_crate_tests() {
    let mut rt = MockRuntime::default();
    let mut cx = Context::<MockEngine>::new(<MockEngine as crate::Engine>::enter(&mut rt));
    let ctx: &mut MockContext<'_> = __cx::context_mut(&mut cx);
    let _ = ctx;
}

#[test]
fn promise_bridge_accepts_hrtb_to_js() {
    fn assert_hrtb<T>()
    where
        T: for<'a> crate::ToJs<'a, MockEngine>,
    {
    }
    assert_hrtb::<u32>();
}

#[test]
fn from_js_u32_conversion_errors() {
    use crate::FromJs;

    let mut rt = MockRuntime::default();
    let mut cx = Context::new(<MockEngine as crate::Engine>::enter(&mut rt));
    let ok = u32::from_js(&mut cx, MockValue::number(7)).expect("parse");
    assert_eq!(ok, 7u32);
    let err = u32::from_js(&mut cx, MockValue::number(999)).unwrap_err();
    match err {
        JsError::TypeError(_) => {}
        other => panic!("expected TypeError, got {other:?}"),
    }
}
