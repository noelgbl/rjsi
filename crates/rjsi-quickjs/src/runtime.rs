use rjsi_core::{
    Context, InternKey, JsResult, Key, KeyCache, MicrotaskDrainPolicy, Runtime, StaticKeySlot
};
use rquickjs::{Atom, Context as QContext, Ctx, Runtime as QRuntime};

use crate::engine::{QuickJsEngine, map_err};

pub struct QuickJsRuntime {
    #[allow(dead_code)]
    pub(crate) rt: QRuntime,
    pub(crate) ctx: QContext,
    microtask_policy: MicrotaskDrainPolicy,
    static_slots: Vec<Option<String>>,
}

impl QuickJsRuntime {
    pub fn new() -> Self {
        let rt = QRuntime::new().unwrap();
        let ctx = QContext::full(&rt).unwrap();
        Self {
            rt,
            ctx,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            static_slots: Vec::new(),
        }
    }
}

impl Default for QuickJsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime<QuickJsEngine> for QuickJsRuntime {
    fn with_scope<R>(
        &mut self,
        f: impl for<'rt> FnOnce(&mut Context<'rt, QuickJsEngine>) -> R,
    ) -> R {
        self.ctx.clone().with(|qctx: Ctx<'_>| {
            let mut cx = Context::new(qctx);
            f(&mut cx)
        })
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.microtask_policy
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.microtask_policy = policy;
    }
}

impl InternKey<QuickJsEngine> for QuickJsRuntime {
    fn intern_str<'cx>(
        &mut self,
        cx: &mut Context<'cx, QuickJsEngine>,
        s: &str,
    ) -> JsResult<'cx, QuickJsEngine, Key<'cx, QuickJsEngine>> {
        let cx_raw = rjsi_core::__cx::context_mut(cx);
        let res = Atom::from_str(cx_raw.clone(), s);
        map_err(cx_raw, res).map(|a| Key::new(a))
    }
}

impl KeyCache<QuickJsEngine> for QuickJsRuntime {
    fn get_or_intern<'cx>(
        &mut self,
        cx: &mut Context<'cx, QuickJsEngine>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, QuickJsEngine, Key<'cx, QuickJsEngine>> {
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

        let cx_raw = rjsi_core::__cx::context_mut(cx);
        let res = Atom::from_str(cx_raw.clone(), &s);
        map_err(cx_raw, res).map(|a| Key::new(a))
    }
}
