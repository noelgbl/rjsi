use criterion::{Criterion, criterion_group, criterion_main};
use rjsi_core::{Global, JsRuntime, JsScope, Source};
use rjsi_v8::{V8Engine, V8RuntimeContext};

fn bench_v8_hot_paths(c: &mut Criterion) {
    let runtime = V8RuntimeContext::new();
    c.bench_function("v8_eval", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let _ = scope.eval(Source::from_bytes("1 + 2 + 3"))?;
                    Ok(())
                })
                .unwrap()
        })
    });
    c.bench_function("v8_root_restore", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let value = scope.number(42.0);
                    let global = Global::<V8Engine>::new(scope, &value);
                    let _ = global.get(scope);
                    Ok(())
                })
                .unwrap()
        })
    });
    c.bench_function("v8_local_property_static_key", |b| {
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
    c.bench_function("v8_raw_call", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let function = scope.eval(Source::from_bytes("(x => x + 1)"))?;
                    let arg = scope.number(41.0);
                    let _ = scope.call_function(&function, None, &[arg]).unwrap();
                    Ok(())
                })
                .unwrap()
        })
    });
}
criterion_group!(benches, bench_v8_hot_paths);
criterion_main!(benches);
