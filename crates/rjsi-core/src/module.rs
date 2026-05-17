use crate::Result;
use std::collections::HashMap;

pub trait Resolver: 'static {
    fn resolve(&mut self, base: Option<&str>, name: &str) -> Result<String>;
}

pub trait Loader: 'static {
    fn load(&mut self, name: &str) -> Result<String>;
}

pub struct ModuleHost {
    pub resolver: Box<dyn Resolver>,
    pub loader: Box<dyn Loader>,
}

impl ModuleHost {
    pub fn new<R, L>(resolver: R, loader: L) -> Self
    where
        R: Resolver,
        L: Loader,
    {
        Self {
            resolver: Box::new(resolver),
            loader: Box::new(loader),
        }
    }
}

pub type ImportMetaHook = Box<dyn FnMut(&str) -> HashMap<String, String>>;

pub struct PassthroughResolver;

impl Resolver for PassthroughResolver {
    fn resolve(&mut self, _base: Option<&str>, name: &str) -> Result<String> {
        Ok(name.to_string())
    }
}

#[derive(Default)]
pub struct MapLoader {
    pub map: HashMap<String, String>,
}

impl MapLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(mut self, name: impl Into<String>, source: impl Into<String>) -> Self {
        self.map.insert(name.into(), source.into());
        self
    }

    pub fn insert(&mut self, name: impl Into<String>, source: impl Into<String>) {
        self.map.insert(name.into(), source.into());
    }
}

impl Loader for MapLoader {
    fn load(&mut self, name: &str) -> Result<String> {
        self.map.get(name).cloned().ok_or_else(|| {
            crate::Error::from_js(
                "module specifier",
                "loaded source",
                Some(format!("no module named {name:?} in MapLoader")),
            )
        })
    }
}
