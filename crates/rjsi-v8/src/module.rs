use std::collections::{HashMap, VecDeque};
use std::pin::pin;
use std::sync::Mutex;

use rjsi_core::module::{ImportMetaHook, ModuleHost};
use rjsi_core::{Context, Error, Result};

use crate::engine::{V8Engine, cast_local, get_scope};

pub(crate) struct ModuleStateSlot(pub(crate) Mutex<ModuleState>);

pub(crate) struct ModuleState {
    pub(crate) host: Option<ModuleHost>,
    pub(crate) meta_hook: Option<ImportMetaHook>,
    pub(crate) by_name: HashMap<String, v8::Global<v8::Module>>,
    pub(crate) name_by_identity: HashMap<i32, String>,
}

impl ModuleState {
    fn new() -> Self {
        Self {
            host: None,
            meta_hook: None,
            by_name: HashMap::new(),
            name_by_identity: HashMap::new(),
        }
    }
}

fn ensure_state_slot(isolate: &mut v8::Isolate) {
    if isolate.get_slot::<ModuleStateSlot>().is_none() {
        isolate.set_slot(ModuleStateSlot(Mutex::new(ModuleState::new())));
    }
}

fn with_state<R>(isolate: &v8::Isolate, f: impl FnOnce(&mut ModuleState) -> R) -> Option<R> {
    let slot = isolate.get_slot::<ModuleStateSlot>()?;
    let mut guard = slot.0.lock().ok()?;
    Some(f(&mut guard))
}

fn make_script_origin<'s>(scope: &v8::PinScope<'s, '_>, name: &str) -> v8::ScriptOrigin<'s> {
    let resource_name = v8::String::new(scope, name).unwrap();
    v8::ScriptOrigin::new(
        scope,
        resource_name.into(),
        0,
        0,
        false,
        0,
        None,
        false,
        false,
        true, // is_module
        None,
    )
}

fn compile_graph<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root_name: &str,
    root_src: &str,
) -> Result<v8::Local<'s, v8::Module>> {
    let mut queue: VecDeque<(String, String)> = VecDeque::new();
    queue.push_back((root_name.to_string(), root_src.to_string()));

    let mut root_local: Option<v8::Local<'s, v8::Module>> = None;

    while let Some((name, src)) = queue.pop_front() {
        let already = with_state(&**scope, |s| s.by_name.contains_key(&name)).unwrap_or(false);
        if already {
            if name == root_name && root_local.is_none() {
                let global = with_state(&**scope, |s| s.by_name.get(&name).cloned()).flatten();
                if let Some(g) = global {
                    root_local = Some(v8::Local::new(scope, &g));
                }
            }
            continue;
        }

        let code = v8::String::new(scope, &src).ok_or_else(|| {
            Error::type_err(format!("failed to create source string for {name:?}"))
        })?;
        let origin = make_script_origin(scope, &name);
        let mut source = v8::script_compiler::Source::new(code, Some(&origin));

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();

        let module = match v8::script_compiler::compile_module(&try_catch, &mut source) {
            Some(m) => m,
            None => {
                return Err(Error::from_js(
                    "module source",
                    "v8 module",
                    Some(format!("failed to compile module {name:?}")),
                ));
            }
        };

        let requests = module.get_module_requests();
        let n = requests.length();
        let mut deps: Vec<(String, String)> = Vec::new();
        for i in 0..n {
            let item = requests
                .get(&try_catch, i)
                .ok_or_else(|| Error::type_err("failed to read module request"))?;
            let req = v8::Local::<v8::ModuleRequest>::try_from(item)
                .map_err(|_| Error::type_err("module request is not a ModuleRequest"))?;
            let spec_local = req.get_specifier();
            let isolate_ref: &v8::Isolate = &**try_catch;
            let specifier = spec_local.to_rust_string_lossy(isolate_ref);

            let resolved = with_state(isolate_ref, |s| {
                let host = s.host.as_mut().ok_or_else(|| {
                    Error::type_err("no module host installed; call install_module_host first")
                })?;
                host.resolver.resolve(Some(&name), &specifier)
            })
            .unwrap_or_else(|| Err(Error::type_err("module state missing")))?;

            let known =
                with_state(isolate_ref, |s| s.by_name.contains_key(&resolved)).unwrap_or(false);
            if known {
                continue;
            }

            let src = with_state(isolate_ref, |s| {
                let host = s.host.as_mut().ok_or_else(|| {
                    Error::type_err("no module host installed; call install_module_host first")
                })?;
                host.loader.load(&resolved)
            })
            .unwrap_or_else(|| Err(Error::type_err("module state missing")))?;

            deps.push((resolved, src));
        }

        let identity = module.get_identity_hash().get();
        {
            let isolate: &mut v8::Isolate = &mut **try_catch;
            let global = v8::Global::new(isolate, module);
            with_state(&*isolate, |s| {
                s.by_name.insert(name.clone(), global);
                s.name_by_identity.insert(identity, name.clone());
            });
        }

        if name == root_name {
            root_local = Some(unsafe { cast_local(module) });
        }

        for d in deps {
            queue.push_back(d);
        }
    }

    root_local.ok_or_else(|| Error::type_err("failed to compile root module"))
}

fn resolve_module_callback<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    _import_attributes: v8::Local<'s, v8::FixedArray>,
    referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
    let scope = pin!(unsafe { v8::CallbackScope::new(context) });
    let mut scope = scope.init();

    let isolate_ref: &v8::Isolate = &**scope;
    let spec = specifier.to_rust_string_lossy(isolate_ref);
    let referrer_id = referrer.get_identity_hash().get();

    let referrer_name = with_state(isolate_ref, |s| {
        s.name_by_identity.get(&referrer_id).cloned()
    })
    .flatten()?;

    let resolved = with_state(isolate_ref, |s| {
        let host = s.host.as_mut()?;
        host.resolver.resolve(Some(&referrer_name), &spec).ok()
    })
    .flatten()?;

    let global = with_state(isolate_ref, |s| s.by_name.get(&resolved).cloned()).flatten()?;
    Some(v8::Local::new(&mut scope, &global))
}

unsafe extern "C" fn import_meta_callback<'a, 'b, 'c>(
    context: v8::Local<'a, v8::Context>,
    module: v8::Local<'b, v8::Module>,
    meta: v8::Local<'c, v8::Object>,
) {
    let scope = pin!(unsafe { v8::CallbackScope::new(context) });
    let mut scope = scope.init();

    let isolate_ref: &v8::Isolate = &**scope;
    let id = module.get_identity_hash().get();
    let name = match with_state(isolate_ref, |s| s.name_by_identity.get(&id).cloned()).flatten() {
        Some(n) => n,
        None => return,
    };

    let props = match with_state(isolate_ref, |s| {
        s.meta_hook.as_mut().map(|hook| hook(&name))
    })
    .flatten()
    {
        Some(p) => p,
        None => return,
    };

    for (k, v) in props.into_iter() {
        let key = match v8::String::new(&mut scope, &k) {
            Some(s) => s,
            None => continue,
        };
        let val = match v8::String::new(&mut scope, &v) {
            Some(s) => s,
            None => continue,
        };
        let _ = meta.create_data_property(&mut scope, key.into(), val.into());
    }
}

fn dynamic_import_callback<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    _host_defined_options: v8::Local<'s, v8::Data>,
    _resource_name: v8::Local<'s, v8::Value>,
    specifier: v8::Local<'s, v8::String>,
    _import_attributes: v8::Local<'s, v8::FixedArray>,
) -> Option<v8::Local<'s, v8::Promise>> {
    let resolver = v8::PromiseResolver::new(scope)?;
    let promise = resolver.get_promise(scope);

    let isolate_ref: &v8::Isolate = &**scope;
    let spec_str = specifier.to_rust_string_lossy(isolate_ref);

    let resolved = match with_state(isolate_ref, |s| {
        let host = s.host.as_mut()?;
        host.resolver.resolve(None, &spec_str).ok()
    })
    .flatten()
    {
        Some(r) => r,
        None => {
            let msg = v8::String::new(scope, "module host failed to resolve")?;
            let exc = v8::Exception::error(scope, msg);
            let _ = resolver.reject(scope, exc);
            return Some(promise);
        }
    };

    let src = match with_state(isolate_ref, |s| {
        let host = s.host.as_mut()?;
        host.loader.load(&resolved).ok()
    })
    .flatten()
    {
        Some(s) => s,
        None => {
            let msg = v8::String::new(scope, "module host failed to load")?;
            let exc = v8::Exception::error(scope, msg);
            let _ = resolver.reject(scope, exc);
            return Some(promise);
        }
    };

    let module = match compile_graph(scope, &resolved, &src) {
        Ok(m) => m,
        Err(e) => {
            let msg = v8::String::new(scope, &e.to_string())?;
            let exc = v8::Exception::error(scope, msg);
            let _ = resolver.reject(scope, exc);
            return Some(promise);
        }
    };

    if module
        .instantiate_module(scope, resolve_module_callback)
        .is_none()
    {
        let msg = v8::String::new(scope, "failed to instantiate module")?;
        let exc = v8::Exception::error(scope, msg);
        let _ = resolver.reject(scope, exc);
        return Some(promise);
    }

    if module.evaluate(scope).is_none() {
        let msg = v8::String::new(scope, "failed to evaluate module")?;
        let exc = v8::Exception::error(scope, msg);
        let _ = resolver.reject(scope, exc);
        return Some(promise);
    }

    let namespace = module.get_module_namespace();
    let _ = resolver.resolve(scope, namespace);
    Some(promise)
}

impl rjsi_core::capabilities::Modules for V8Engine {
    fn install_module_host(runtime: &mut Self::Runtime, host: ModuleHost) -> Result<()> {
        runtime.with_isolate_mut(|isolate| {
            ensure_state_slot(isolate);
            with_state(&*isolate, |s| {
                s.host = Some(host);
            });
            isolate.set_host_import_module_dynamically_callback(dynamic_import_callback);
            isolate.set_host_initialize_import_meta_object_callback(import_meta_callback);
        });
        Ok(())
    }

    fn set_import_meta_hook(runtime: &mut Self::Runtime, hook: ImportMetaHook) -> Result<()> {
        runtime.with_isolate_mut(|isolate| {
            ensure_state_slot(isolate);
            with_state(&*isolate, |s| {
                s.meta_hook = Some(hook);
            });
            isolate.set_host_initialize_import_meta_object_callback(import_meta_callback);
        });
        Ok(())
    }

    fn module_evaluate<'js>(
        cx: &mut Context<'js, Self>,
        name: &str,
        src: &str,
    ) -> Result<Self::Object<'js>> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        {
            let isolate: &mut v8::Isolate = &mut **scope;
            ensure_state_slot(isolate);
        }

        let module = compile_graph(scope, name, src)?;
        if module
            .instantiate_module(scope, resolve_module_callback)
            .is_none()
        {
            return Err(Error::type_err("failed to instantiate module"));
        }
        let result = module
            .evaluate(scope)
            .ok_or_else(|| Error::type_err("failed to evaluate module"))?;
        let obj: v8::Local<'_, v8::Object> = result
            .try_into()
            .map_err(|_| Error::type_err("module evaluation did not return a promise"))?;
        Ok(unsafe { cast_local(obj) })
    }

    fn module_import<'js>(
        cx: &mut Context<'js, Self>,
        specifier: &str,
    ) -> Result<Self::Object<'js>> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        {
            let isolate: &mut v8::Isolate = &mut **scope;
            ensure_state_slot(isolate);
        }

        let isolate_ref: &v8::Isolate = &**scope;
        let resolved = with_state(isolate_ref, |s| {
            let host = s.host.as_mut().ok_or_else(|| {
                Error::type_err("no module host installed; call install_module_host first")
            })?;
            host.resolver.resolve(None, specifier)
        })
        .unwrap_or_else(|| Err(Error::type_err("module state missing")))?;

        let src = with_state(isolate_ref, |s| {
            let host = s.host.as_mut().ok_or_else(|| {
                Error::type_err("no module host installed; call install_module_host first")
            })?;
            host.loader.load(&resolved)
        })
        .unwrap_or_else(|| Err(Error::type_err("module state missing")))?;

        let module = compile_graph(scope, &resolved, &src)?;
        if module
            .instantiate_module(scope, resolve_module_callback)
            .is_none()
        {
            return Err(Error::type_err("failed to instantiate module"));
        }

        let resolver = v8::PromiseResolver::new(scope)
            .ok_or_else(|| Error::type_err("failed to create promise"))?;
        let promise = resolver.get_promise(scope);

        if module.evaluate(scope).is_none() {
            return Err(Error::type_err("failed to evaluate module"));
        }

        let namespace = module.get_module_namespace();
        let _ = resolver.resolve(scope, namespace);
        let obj: v8::Local<'_, v8::Object> = promise.into();
        Ok(unsafe { cast_local(obj) })
    }
}

impl rjsi_core::RuntimeModulesExt<V8Engine> for crate::runtime::V8Runtime {
    fn install_module_host<R, L>(&mut self, resolver: R, loader: L) -> Result<()>
    where
        R: rjsi_core::module::Resolver,
        L: rjsi_core::module::Loader,
    {
        <V8Engine as rjsi_core::capabilities::Modules>::install_module_host(
            self,
            ModuleHost::new(resolver, loader),
        )
    }

    fn install_module_host_boxed(&mut self, host: ModuleHost) -> Result<()> {
        <V8Engine as rjsi_core::capabilities::Modules>::install_module_host(self, host)
    }

    fn set_import_meta_hook<F>(&mut self, hook: F) -> Result<()>
    where
        F: FnMut(&str) -> std::collections::HashMap<String, String> + 'static,
    {
        <V8Engine as rjsi_core::capabilities::Modules>::set_import_meta_hook(self, Box::new(hook))
    }
}
