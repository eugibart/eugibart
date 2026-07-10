//! Loads and indexes ECU definition files.

use std::collections::BTreeMap;
use std::path::Path;

use crate::schema::EcuDefinition;
use crate::{DefsError, Result};

#[derive(Debug, Default)]
pub struct Registry {
    defs: BTreeMap<String, EcuDefinition>,
}

impl Registry {
    /// Recursively load every `*.toml` under `dir`.
    pub fn load_dir(dir: &Path) -> Result<Self> {
        let mut registry = Self::default();
        registry.load_dir_into(dir)?;
        Ok(registry)
    }

    fn load_dir_into(&mut self, dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir).map_err(|source| DefsError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| DefsError::Io {
                path: dir.display().to_string(),
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir_into(&path)?;
            } else if path.extension().is_some_and(|ext| ext == "toml") {
                let text = std::fs::read_to_string(&path).map_err(|source| DefsError::Io {
                    path: path.display().to_string(),
                    source,
                })?;
                self.add_toml(&text, &path.display().to_string())?;
            }
        }
        Ok(())
    }

    pub fn add_toml(&mut self, text: &str, origin: &str) -> Result<()> {
        let def: EcuDefinition = toml::from_str(text).map_err(|source| DefsError::Parse {
            path: origin.to_string(),
            source: Box::new(source),
        })?;
        def.validate().map_err(|reason| DefsError::Invalid {
            id: def.ecu.id.clone(),
            reason,
        })?;
        let id = def.ecu.id.clone();
        if self.defs.contains_key(&id) {
            return Err(DefsError::DuplicateId(id));
        }
        self.defs.insert(id, def);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&EcuDefinition> {
        self.defs.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EcuDefinition> {
        self.defs.values()
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}
