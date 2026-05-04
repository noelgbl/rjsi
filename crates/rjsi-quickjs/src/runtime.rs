use std::collections::HashMap;

use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result as RjsiResult, Runtime};
use rquickjs::{Atom, Context as QContext, Ctx, Runtime as QRuntime};

use crate::engine::{QuickJsContext, QuickJsEngine, map_err};

pub struct QuickJsPreparedKeyData {
    atom: Atom<'static>,
}

pub struct QuickJsRuntime {
    prepared_keys: HashMap<u64, QuickJsPreparedKeyData>,
    #[allow(dead_code)]
    pub(crate) rt: QRuntime,
    pub(crate) ctx: QContext,
    microtask_policy: MicrotaskDrainPolicy,
}

impl QuickJsRuntime {
    pub fn new() -> Self {
        let rt = QRuntime::new().unwrap();
        let ctx = QContext::full(&rt).unwrap();
        Self {
            prepared_keys: HashMap::new(),
            rt,
            ctx,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        }
    }

    pub fn prepare_key(
        &mut self,
        name: impl Into<String>,
    ) -> anyhow::Result<PreparedKey<QuickJsEngine>> {
        let key = PreparedKey::new(name);
        self.ensure_prepared_key(key.id(), key.as_str())
            .map_err(anyhow::Error::new)?;
        Ok(key)
    }

    fn ensure_prepared_key(&mut self, id: u64, name: &str) -> Result<(), rquickjs::Error> {
        if self.prepared_keys.contains_key(&id) {
            return Ok(());
        }

        self.ctx.clone().with(|qctx: Ctx<'_>| {
            let atom = Atom::from_str(qctx, name)?;
            self.prepared_keys.insert(
                id,
                QuickJsPreparedKeyData {
                    atom: unsafe { std::mem::transmute(atom) },
                },
            );
            Ok(())
        })
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
        let runtime = self as *mut _;
        self.ctx.clone().with(|qctx: Ctx<'_>| {
            let mut cx = Context::new(QuickJsContext { qctx, runtime });
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

pub(crate) fn prepared_key<'cx>(
    cx: &mut QuickJsContext<'cx>,
    key: &PreparedKey<QuickJsEngine>,
) -> RjsiResult<Atom<'cx>> {
    let runtime = unsafe { &mut *cx.runtime };
    if !runtime.prepared_keys.contains_key(&key.id()) {
        let atom = Atom::from_str(cx.qctx.clone(), key.as_str());
        let atom = map_err(cx, atom)?;
        runtime.prepared_keys.insert(
            key.id(),
            QuickJsPreparedKeyData {
                atom: unsafe { std::mem::transmute(atom) },
            },
        );
    }
    let atom = &runtime.prepared_keys.get(&key.id()).unwrap().atom;
    Ok(unsafe { std::mem::transmute(atom.clone()) })
}

pub(crate) fn _map_prepare_err<'cx, T>(
    cx: &QuickJsContext<'cx>,
    res: rquickjs::Result<T>,
) -> RjsiResult<T> {
    map_err(cx, res)
}
