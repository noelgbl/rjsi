use std::path::Path;

use crate::{HostError, JsResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    JavaScript(Vec<u8>),
    ByteCode(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    kind: SourceKind,
    name: Option<String>,
}

impl Source {
    pub fn from_bytes<T: AsRef<[u8]>>(code: T) -> Self {
        Self {
            kind: SourceKind::JavaScript(code.as_ref().to_vec()),
            name: None,
        }
    }

    pub fn from_bytecode(code: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: SourceKind::ByteCode(code.into()),
            name: None,
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> JsResult<Self> {
        let code = std::fs::read(path.as_ref())?;
        let kind = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("js") => SourceKind::JavaScript(code),
            Some("rjsi") => SourceKind::ByteCode(code),
            _ => {
                return Err(HostError::new(
                    crate::error::E_NOT_SUPPORTED,
                    format!("Unsupported source file type: {}", path.as_ref().display()),
                )
                .into());
            }
        };
        Ok(Self {
            kind,
            name: Some(path.as_ref().to_string_lossy().into_owned()),
        })
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn kind(&self) -> &SourceKind {
        &self.kind
    }

    pub fn code(&self) -> &[u8] {
        match &self.kind {
            SourceKind::JavaScript(code) | SourceKind::ByteCode(code) => code,
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn len(&self) -> usize {
        self.code().len()
    }

    pub fn is_empty(&self) -> bool {
        self.code().is_empty()
    }
}
