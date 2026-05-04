use std::collections::HashSet;

use rjsi_core::{Context, JsResult, MicrotaskDrainPolicy, PreparedKey, Runtime};
use rusty_hermes::{PropNameId, Runtime as HermesRtInner};

use crate::engine::{HermesEngine, runtime_ffi_ptr};

/// Marker type for [`HermesEngine::PreparedKeyData`](crate::engine::HermesEngine).
#[derive(Clone, Copy, Debug, Default)]
pub struct HermesPreparedKeyData;

pub struct HermesRuntime {
    /// IDs registered via [`HermesRuntime::prepare_key`].
    prepared_keys: HashSet<u64>,
    pub inner: HermesRtInner,
    microtask_policy: MicrotaskDrainPolicy,
}

impl HermesRuntime {
    pub fn new() -> Result<Self, rusty_hermes::Error> {
        Ok(Self {
            prepared_keys: HashSet::new(),
            inner: HermesRtInner::new()?,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        })
    }

    pub fn with_config(config: rusty_hermes::RuntimeConfig) -> Result<Self, rusty_hermes::Error> {
        Ok(Self {
            prepared_keys: HashSet::new(),
            inner: HermesRtInner::with_config(config)?,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        })
    }

    pub fn prepare_key(
        &mut self,
        name: impl Into<String>,
    ) -> anyhow::Result<PreparedKey<HermesEngine>> {
        let key = PreparedKey::new(name);
        self.ensure_prepared_key(key.id(), key.as_str());
        Ok(key)
    }

    fn ensure_prepared_key(&mut self, id: u64, _name: &str) {
        self.prepared_keys.insert(id);
    }
}

impl Default for HermesRuntime {
    fn default() -> Self {
        Self::new().expect("Hermes runtime creation failed")
    }
}

impl Runtime<HermesEngine> for HermesRuntime {
    fn with_scope<R>(
        &mut self,
        f: impl for<'rt> FnOnce(&mut Context<'rt, HermesEngine>) -> R,
    ) -> R {
        let runtime_ptr = self as *mut _;
        let ctx = crate::engine::HermesContext {
            inner: &mut self.inner,
            runtime: runtime_ptr,
        };
        let mut cx = Context::new(ctx);
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
    cx: &mut crate::engine::HermesContext<'cx>,
    key: &PreparedKey<HermesEngine>,
) -> JsResult<PropNameId<'cx>> {
    if cx.runtime.is_null() {
        let p = PropNameId::from_utf8(&*cx.inner, key.as_str());
        return Ok(unsafe { std::mem::transmute(p) });
    }

    let runtime = unsafe { &mut *cx.runtime };
    runtime.ensure_prepared_key(key.id(), key.as_str());
    let p = PropNameId::from_utf8(&*cx.inner, key.as_str());
    Ok(unsafe { std::mem::transmute(p) })
}

impl HermesRuntime {
    pub fn raw_hermes_rt(&self) -> *mut libhermes_sys::HermesRt {
        runtime_ffi_ptr(&self.inner)
    }
}
