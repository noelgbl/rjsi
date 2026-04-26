use criterion::{Criterion, criterion_group, criterion_main};
use rjsi_core::{ContextLike, Global, ScopeLike, ValueLike};
use rjsi_v8::{V8Runtime, V8RuntimeContext};

fn bench_v8_hot_paths(c: &mut Criterion) {
    let runtime = V8RuntimeContext::new();
    c.bench_function("v8_eval", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let _ = scope.eval("1 + 2 + 3")?;
                    Ok(())
                })
                .unwrap();
        })
    });

    c.bench_function("v8_root_restore", |b| {
        b.iter(|| {
            let global = runtime
                .with_scope(|scope| {
                    let value = scope.number(42.0);
                    Ok(Global::<V8Runtime>::new(scope, value))
                })
                .unwrap();
            runtime
                .with_scope(|scope| {
                    let restored = global.get(scope);
                    let _ = restored.as_f64(scope);
                    Ok(())
                })
                .unwrap();
        })
    });

    c.bench_function("v8_property_set", |b| {
        b.iter(|| {
            runtime
                .with_scope(|scope| {
                    let object = scope.object();
                    let value = scope.number(42.0);
                    object.set(scope, "answer", value);
                    Ok(())
                })
                .unwrap();
        })
    });
}

criterion_group!(benches, bench_v8_hot_paths);
criterion_main!(benches);
