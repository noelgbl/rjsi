use std::cell::RefCell;
use std::rc::Rc;

use rjsi_core::module::ImportMetaHook;
use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result as RjsiResult, Runtime, Store};
use rquickjs::{Atom, Context as QContext, Ctx, Runtime as QRuntime};

use crate::engine::{QuickJsContext, QuickJsEngine, map_err};

pub struct QuickJsPreparedKeyData {
    atom: Atom<'static>,
}

impl QuickJsPreparedKeyData {
    pub(crate) fn atom(&self) -> &Atom<'static> {
        &self.atom
    }
}

pub(crate) type ImportMetaHookCell = Rc<RefCell<Option<ImportMetaHook>>>;

pub struct QuickJsRuntime {
    pub(crate) store: Store<QuickJsEngine>,
    #[allow(dead_code)]
    pub(crate) rt: QRuntime,
    pub(crate) ctx: QContext,
    pub(crate) import_meta_hook: ImportMetaHookCell,
}

impl QuickJsRuntime {
    pub fn new() -> Self {
        let rt = QRuntime::new().unwrap();
        let ctx = QContext::full(&rt).unwrap();
        Self {
            store: Store::new(),
            rt,
            ctx,
            import_meta_hook: Rc::new(RefCell::new(None)),
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
        if self.store.contains_prepared_key(id) {
            return Ok(());
        }

        self.ctx.clone().with(|qctx: Ctx<'_>| {
            let atom = Atom::from_str(qctx, name)?;
            self.store.insert_prepared_key(
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
        f: impl for<'js> FnOnce(&mut Context<'js, QuickJsEngine>) -> R,
    ) -> R {
        let runtime = self as *mut _;
        self.ctx.clone().with(|qctx: Ctx<'_>| {
            let mut cx = Context::new(QuickJsContext { qctx, runtime });
            f(&mut cx)
        })
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.store.microtask_policy()
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.store.set_microtask_policy(policy);
    }
}

pub(crate) fn prepared_key<'js>(
    cx: &mut QuickJsContext<'js>,
    key: &PreparedKey<QuickJsEngine>,
) -> RjsiResult<Atom<'js>> {
    let runtime = unsafe { &mut *cx.runtime };
    if !runtime.store.contains_prepared_key(key.id()) {
        let atom = Atom::from_str(cx.qctx.clone(), key.as_str());
        let atom = map_err(cx, atom)?;
        runtime.store.insert_prepared_key(
            key.id(),
            QuickJsPreparedKeyData {
                atom: unsafe { std::mem::transmute(atom) },
            },
        );
    }
    let prepared = runtime.store.get_prepared_key(key.id()).unwrap();
    Ok(unsafe { std::mem::transmute(prepared.atom().clone()) })
}

pub(crate) fn _map_prepare_err<'js, T>(
    cx: &QuickJsContext<'js>,
    res: rquickjs::Result<T>,
) -> RjsiResult<T> {
    map_err(cx, res)
}
