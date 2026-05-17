use rjsi_core::module::{ImportMetaHook, Loader as RjsiLoader, ModuleHost, Resolver as RjsiResolver};
use rjsi_core::{Context, Error, Result};
use rquickjs::loader::{Loader as QLoader, Resolver as QResolver};
use rquickjs::module::{Declared, Module as QModule};
use rquickjs::{Ctx, Error as QError};

use crate::engine::{QuickJsEngine, map_err};
use crate::runtime::QuickJsRuntime;

struct ResolverAdapter {
    inner: Box<dyn RjsiResolver>,
}

impl QResolver for ResolverAdapter {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
    ) -> rquickjs::Result<String> {
        let base_opt = if base.is_empty() { None } else { Some(base) };
        self.inner
            .resolve(base_opt, name)
            .map_err(|e| QError::new_resolving_message(base, name, e.to_string()))
    }
}

struct LoaderAdapter {
    inner: Box<dyn RjsiLoader>,
}

impl QLoader for LoaderAdapter {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<QModule<'js, Declared>> {
        let src = self
            .inner
            .load(name)
            .map_err(|e| QError::new_loading_message(name, e.to_string()))?;
        QModule::declare(ctx.clone(), name, src)
    }
}

impl rjsi_core::capabilities::Modules for QuickJsEngine {
    fn install_module_host(
        runtime: &mut Self::Runtime,
        host: ModuleHost,
    ) -> Result<()> {
        runtime.rt.set_loader(
            ResolverAdapter {
                inner: host.resolver,
            },
            LoaderAdapter { inner: host.loader },
        );
        Ok(())
    }

    /// TODO: How does this work in QJS?
    fn set_import_meta_hook(
        _runtime: &mut Self::Runtime,
        _hook: ImportMetaHook,
    ) -> Result<()> {
        Err(Error::from_js(
            "import.meta hook",
            "QuickJS",
            Some(
                "QuickJS does support import.meta rn"
                    .to_string(),
            ),
        ))
    }

    fn module_evaluate<'rt>(
        cx: &mut Context<'rt, Self>,
        name: &str,
        src: &str,
    ) -> Result<Self::Object<'rt>> {
        let qjs_cx = rjsi_core::__cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let res = QModule::evaluate(qctx, name.as_bytes().to_vec(), src.as_bytes().to_vec());
        let promise = map_err(qjs_cx, res)?;
        Ok(promise.into_value().into_object().unwrap())
    }

    fn module_import<'rt>(
        cx: &mut Context<'rt, Self>,
        specifier: &str,
    ) -> Result<Self::Object<'rt>> {
        let qjs_cx = rjsi_core::__cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let res = QModule::import(&qctx, specifier.as_bytes().to_vec());
        let promise = map_err(qjs_cx, res)?;
        Ok(promise.into_value().into_object().unwrap())
    }
}

impl rjsi_core::RuntimeModulesExt<QuickJsEngine> for QuickJsRuntime {
    fn install_module_host<R, L>(&mut self, resolver: R, loader: L) -> Result<()>
    where
        R: RjsiResolver,
        L: RjsiLoader,
    {
        <QuickJsEngine as rjsi_core::capabilities::Modules>::install_module_host(
            self,
            ModuleHost::new(resolver, loader),
        )
    }

    fn install_module_host_boxed(&mut self, host: ModuleHost) -> Result<()> {
        <QuickJsEngine as rjsi_core::capabilities::Modules>::install_module_host(self, host)
    }

    fn set_import_meta_hook<F>(&mut self, hook: F) -> Result<()>
    where
        F: FnMut(&str) -> std::collections::HashMap<String, String> + 'static,
    {
        <QuickJsEngine as rjsi_core::capabilities::Modules>::set_import_meta_hook(
            self,
            Box::new(hook),
        )
    }
}
