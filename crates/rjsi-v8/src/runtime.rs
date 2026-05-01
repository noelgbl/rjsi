use rjsi_core::{Context, InternKey, JsResult, Key, KeyCache, MicrotaskDrainPolicy, Runtime, StaticKeySlot};

pub struct V8Runtime {
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    microtask_policy: MicrotaskDrainPolicy,
    static_slots: Vec<Option<String>>,
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
            isolate,
            context,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            static_slots: Vec::new(),
        }
    }
}

impl Default for V8Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime<crate::engine::V8Engine> for V8Runtime {
    fn with_scope<R>(&mut self, f: impl for<'rt> FnOnce(&mut Context<'rt, crate::engine::V8Engine>) -> R) -> R {
        let scope1 = v8::HandleScope::new(&mut self.isolate);
        let scope1_pin = std::pin::pin!(scope1);
        let mut handle_scope = scope1_pin.init();

        let ctx = v8::Local::new(&mut handle_scope, &self.context);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, ctx);
        
        let cx_raw = crate::engine::V8Context {
            scope: &mut context_scope as *mut _ as *mut std::ffi::c_void,
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

impl InternKey<crate::engine::V8Engine> for V8Runtime {
    fn intern_str<'cx>(
        &mut self,
        cx: &mut Context<'cx, crate::engine::V8Engine>,
        s: &str,
    ) -> JsResult<'cx, crate::engine::V8Engine, Key<'cx, crate::engine::V8Engine>> {
        let cx_raw = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { crate::engine::get_scope(cx_raw) };
        let string = v8::String::new(scope, s).unwrap();
        let name: v8::Local<'_, v8::Name> = string.into();
        Ok(Key::new(unsafe { crate::engine::cast_local(name) }))
    }
}

impl KeyCache<crate::engine::V8Engine> for V8Runtime {
    fn get_or_intern<'cx>(
        &mut self,
        cx: &mut Context<'cx, crate::engine::V8Engine>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, crate::engine::V8Engine, Key<'cx, crate::engine::V8Engine>> {
        let idx = slot.0 as usize;
        if idx >= self.static_slots.len() {
            self.static_slots.resize(idx + 1, None);
        }
        
        let s = if let Some(s) = &self.static_slots[idx] {
            s.clone()
        } else {
            let new_s = format!("__static_slot_{}", idx);
            self.static_slots[idx] = Some(new_s.clone());
            new_s
        };

        self.intern_str(cx, &s)
    }
}
