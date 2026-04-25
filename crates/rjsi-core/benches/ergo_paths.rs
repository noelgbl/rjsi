use criterion::{criterion_group, criterion_main, Criterion};
use rjsi_core::convert::ScopeExt;
use rjsi_core::{JsRuntime, JsScope, Source};
use rjsi_quickjs::QuickJsRuntimeContext;

fn bench_ergo_paths(c: &mut Criterion) {
    let runtime = QuickJsRuntimeContext::new();

    c.bench_function("ergo_raw_property_get_set", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let object = scope.eval(Source::from_bytes("({})"))?;
                    let key = scope.static_property_key("answer");
                    let value = scope.number(42.0);
                    scope.set_property(&object, &key, &value).unwrap();
                    let _ = scope.get_property(&object, &key).unwrap();
                    Ok(())
                })
                .unwrap()
        })
    });

    c.bench_function("ergo_ext_property_get_set", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let object = scope.eval(Source::from_bytes("({})"))?;
                    scope.set(&object, "answer", 42.0).unwrap();
                    let _ = scope.get(&object, "answer").unwrap();
                    Ok(())
                })
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_ergo_paths);
criterion_main!(benches);
