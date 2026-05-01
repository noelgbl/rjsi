use std::ops::DerefMut;

use boa_engine::{Context as BoaCx, JsString};
use rjsi_core::{
    Context, InternKey, JsResult, Key, KeyCache, MicrotaskDrainPolicy, Runtime, StaticKeySlot,
};

use crate::engine::BoaEngine;

pub struct BoaRuntime {
    pub(crate) context: BoaCx,
    microtask_policy: MicrotaskDrainPolicy,
    static_slots: Vec<Option<String>>,
}

impl BoaRuntime {
    pub fn new() -> Self {
        Self {
            context: BoaCx::default(),
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            static_slots: Vec::new(),
        }
    }
}

impl Default for BoaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime<BoaEngine> for BoaRuntime {
    fn with_scope<R>(&mut self, f: impl for<'rt> FnOnce(&mut Context<'rt, BoaEngine>) -> R) -> R {
        let wrapper = crate::engine::BoaContext {
            inner: &mut self.context,
        };
        let mut cx = Context::new(wrapper);
        f(&mut cx)
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.microtask_policy
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.microtask_policy = policy;
    }
}

impl InternKey<BoaEngine> for BoaRuntime {
    fn intern_str<'cx>(
        &mut self,
        cx: &mut Context<'cx, BoaEngine>,
        s: &str,
    ) -> JsResult<'cx, BoaEngine, Key<'cx, BoaEngine>> {
        let _ = self;
        let boa_cx = rjsi_core::__cx::context_mut(cx).deref_mut();
        let _ = boa_cx.interner_mut().get_or_intern(s);
        Ok(Key::new(JsString::from(s)))
    }
}

impl KeyCache<BoaEngine> for BoaRuntime {
    fn get_or_intern<'cx>(
        &mut self,
        cx: &mut Context<'cx, BoaEngine>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, BoaEngine, Key<'cx, BoaEngine>> {
        let idx = slot.0 as usize;
        if idx >= self.static_slots.len() {
            self.static_slots.resize(idx + 1, None);
        }

        let s = if let Some(stored) = &self.static_slots[idx] {
            stored.clone()
        } else {
            let new_s = format!("__static_slot_{}", idx);
            self.static_slots[idx] = Some(new_s.clone());
            new_s
        };

        let boa_cx = rjsi_core::__cx::context_mut(cx).deref_mut();
        let _ = boa_cx.interner_mut().get_or_intern(s.as_str());
        Ok(Key::new(JsString::from(s.as_str())))
    }
}
