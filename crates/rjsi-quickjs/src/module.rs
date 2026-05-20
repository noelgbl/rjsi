use rjsi_core::module::{
    ImportMetaHook, Loader as RjsiLoader, ModuleHost, Resolver as RjsiResolver
};
use rjsi_core::{Context, Result};
use rquickjs::loader::{Loader as QLoader, Resolver as QResolver};
use rquickjs::module::{Declared, Module as QModule};
use rquickjs::{Ctx, Error as QError};

use crate::engine::{QuickJsEngine, map_err};
use crate::runtime::{ImportMetaHookCell, QuickJsRuntime};

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
    meta_hook: ImportMetaHookCell,
}

impl QLoader for LoaderAdapter {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<QModule<'js, Declared>> {
        let src = self
            .inner
            .load(name)
            .map_err(|e| QError::new_loading_message(name, e.to_string()))?;
        let module = QModule::declare(ctx.clone(), name, src)?;
        populate_import_meta(&module, name, &self.meta_hook)?;
        Ok(module)
    }
}

fn populate_import_meta<'js>(
    module: &QModule<'js, Declared>,
    name: &str,
    hook: &ImportMetaHookCell,
) -> rquickjs::Result<()> {
    let props = hook.borrow_mut().as_mut().map(|h| h(name));
    if let Some(props) = props {
        let meta = module.meta()?;
        for (k, v) in props {
            meta.set(k, v)?;
        }
    }
    Ok(())
}

impl rjsi_core::capabilities::Modules for QuickJsEngine {
    fn install_module_host(runtime: &mut Self::Runtime, host: ModuleHost) -> Result<()> {
        runtime.rt.set_loader(
            ResolverAdapter {
                inner: host.resolver,
            },
            LoaderAdapter {
                inner: host.loader,
                meta_hook: runtime.import_meta_hook.clone(),
            },
        );
        Ok(())
    }

    fn set_import_meta_hook(runtime: &mut Self::Runtime, hook: ImportMetaHook) -> Result<()> {
        *runtime.import_meta_hook.borrow_mut() = Some(hook);
        Ok(())
    }

    fn module_evaluate<'rt>(
        cx: &mut Context<'rt, Self>,
        name: &str,
        src: &str,
    ) -> Result<Self::Object<'rt>> {
        let qjs_cx = rjsi_core::__cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let hook = unsafe { (*qjs_cx.runtime).import_meta_hook.clone() };

        let module = map_err(
            qjs_cx,
            QModule::declare(qctx, name.as_bytes().to_vec(), src.as_bytes().to_vec()),
        )?;
        map_err(qjs_cx, populate_import_meta(&module, name, &hook))?;

        let res = module.eval();
        let (_evaluated, promise) = map_err(qjs_cx, res)?;
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
