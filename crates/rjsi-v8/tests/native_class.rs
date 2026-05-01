use rjsi_core::{
    ClassDescriptor, ClassRegistry, ContextLike, NativeClass, NativeRef, Runtime, class_descriptor
};
use rjsi_v8::{V8Runtime, V8RuntimeContext};

struct Point {
    x: f64,
    y: f64,
}

unsafe fn point_finalizer(p: *mut std::ffi::c_void) {
    unsafe {
        drop(Box::from_raw(p.cast::<Point>()));
    }
}

fn build_point_desc<R: Runtime>() -> ClassDescriptor<R> {
    ClassDescriptor {
        name: "Point",
        constructor: None,
        methods: &[],
        statics: &[],
        accessors: &[],
        symbols: &[],
        finalizer: point_finalizer,
    }
}

unsafe impl NativeClass for Point {
    const NAME: &'static str = "Point";

    fn descriptor<R: Runtime>() -> &'static ClassDescriptor<R> {
        class_descriptor::<R, Point>(build_point_desc)
    }
}

#[test]
fn v8_register_wrap_unwrap() {
    let ctx = V8RuntimeContext::new();
    ctx.with_scope(|scope| {
        <V8RuntimeContext as ClassRegistry<V8Runtime>>::register_class::<Point>(
            &ctx,
            scope,
            Point::descriptor::<V8Runtime>(),
        )?;
        let v = <V8RuntimeContext as ClassRegistry<V8Runtime>>::wrap_native::<Point>(
            scope,
            Point { x: 1.0, y: 2.0 },
        )?;
        let r: NativeRef<'_, Point> =
            <V8RuntimeContext as ClassRegistry<V8Runtime>>::unwrap_native::<Point>(scope, v)
                .expect("unwrap Point");
        assert!((r.get().x - 1.0).abs() < 1e-9);
        assert!((r.get().y - 2.0).abs() < 1e-9);
        Ok(())
    })
    .expect("with_scope");
}
