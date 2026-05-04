use std::collections::HashMap;

use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result, Runtime};

pub struct V8Runtime {
    prepared_keys: HashMap<u64, v8::Global<v8::Name>>,
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    microtask_policy: MicrotaskDrainPolicy,
}

impl V8Runtime {
    pub fn new() -> Self {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        let context = {
            let scope1 = v8::HandleScope::new(&mut isolate);
            let scope1_pin = std::pin::pin!(scope1);
            let mut handle_scope = scope1_pin.init();
            let ctx = v8::Context::new(&mut handle_scope, Default::default());
            v8::Global::new(&mut handle_scope, ctx)
        };

        Self {
            prepared_keys: HashMap::new(),
            isolate,
            context,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        }
    }

    pub fn prepare_key(&mut self, name: impl Into<String>) -> PreparedKey<crate::engine::V8Engine> {
        let key = PreparedKey::new(name);
        self.ensure_prepared_key(key.id(), key.as_str())
            .expect("failed to prepare V8 property name");
        key
    }

    fn ensure_prepared_key(&mut self, id: u64, name: &str) -> std::io::Result<()> {
        if self.prepared_keys.contains_key(&id) {
            return Ok(());
        }

        let scope1 = v8::HandleScope::new(&mut self.isolate);
        let scope1_pin = std::pin::pin!(scope1);
        let mut handle_scope = scope1_pin.init();

        let ctx = v8::Local::new(&mut handle_scope, &self.context);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, ctx);
        let string = v8::String::new(&mut context_scope, name)
            .ok_or_else(|| std::io::Error::other("failed to create V8 property name"))?;
        let local_name: v8::Local<'_, v8::Name> = string.into();
        let global_name = v8::Global::new(&mut context_scope, local_name);
        self.prepared_keys.insert(id, global_name);
        Ok(())
    }
}

impl Default for V8Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime<crate::engine::V8Engine> for V8Runtime {
    fn with_scope<R>(
        &mut self,
        f: impl for<'rt> FnOnce(&mut Context<'rt, crate::engine::V8Engine>) -> R,
    ) -> R {
        let runtime_ptr = self as *mut _;
        let scope1 = v8::HandleScope::new(&mut self.isolate);
        let scope1_pin = std::pin::pin!(scope1);
        let mut handle_scope = scope1_pin.init();

        let ctx = v8::Local::new(&mut handle_scope, &self.context);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, ctx);

        let cx_raw = crate::engine::V8Context {
            scope: &mut context_scope as *mut _ as *mut std::ffi::c_void,
            runtime: runtime_ptr,
            pending_exception: None,
            _phantom: std::marker::PhantomData,
        };
        let mut cx = Context::new(cx_raw);
        f(&mut cx)
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.microtask_policy
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.microtask_policy = policy;
    }
}

pub(crate) fn prepared_key<'cx>(
    cx: &mut crate::engine::V8Context<'cx>,
    key: &PreparedKey<crate::engine::V8Engine>,
) -> Result<v8::Local<'cx, v8::Name>> {
    let scope = unsafe { crate::engine::get_scope(cx) };
    if cx.runtime.is_null() {
        let string = v8::String::new(scope, key.as_str())
            .ok_or_else(|| rjsi_core::Error::type_err("failed to create V8 property name"))?;
        let local_name: v8::Local<'_, v8::Name> = string.into();
        return Ok(unsafe { crate::engine::cast_local(local_name) });
    }

    let runtime = unsafe { &mut *cx.runtime };
    if !runtime.prepared_keys.contains_key(&key.id()) {
        runtime
            .ensure_prepared_key(key.id(), key.as_str())
            .map_err(rjsi_core::Error::from_host)?;
    }
    let global = runtime.prepared_keys.get(&key.id()).unwrap();
    Ok(unsafe { crate::engine::cast_local(v8::Local::new(scope, global)) })
}
