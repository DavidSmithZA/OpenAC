//! Which interface the clients this launcher starts show, remembered across
//! restarts (`launcher-interface.json` in the settings folder). Every client
//! started after a change shows the new one; a running client keeps the
//! one it started with.

use std::path::{Path, PathBuf};

use ac_host_core::json::{self as host_json, ObjectReader};
use ac_host_core::{Interface, PathSet};

use crate::fsutil;
use crate::json::{self, ObjectBuilder, Out};

/// The file's name in the settings folder.
pub const INTERFACE_SETTINGS_FILE_NAME: &str = "launcher-interface.json";

/// Why the setting could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct InterfaceSettingsError(pub String);

/// Reads and writes the interface settings file.
#[derive(Debug, Clone)]
pub struct LauncherInterfaceSettingsStore {
    file_path: PathBuf,
}

impl LauncherInterfaceSettingsStore {
    /// A store for `file_path`.
    pub fn new(file_path: &Path) -> Self {
        Self {
            file_path: fsutil::full_path(file_path),
        }
    }

    /// The store in the settings folder.
    pub fn for_paths(paths: &PathSet) -> Self {
        Self::new(&paths.config_directory.join(INTERFACE_SETTINGS_FILE_NAME))
    }

    /// The file.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Reads the file; a missing file is the retail interface. A file that
    /// is not an object with a known `interface` is an error and is left as
    /// it is.
    pub fn load(&self) -> Result<Interface, InterfaceSettingsError> {
        let bytes = match std::fs::read(&self.file_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Interface::default());
            }
            Err(error) => return Err(InterfaceSettingsError(error.to_string())),
        };
        let invalid = |detail: String| {
            InterfaceSettingsError(format!("The interface settings file is invalid: {detail}"))
        };
        let root = host_json::parse(&bytes).map_err(|error| invalid(error.to_string()))?;
        let mut object = ObjectReader::new(&root).map_err(|error| invalid(error.to_string()))?;
        let name = object
            .take("interface")
            .map(host_json::string_or_null)
            .transpose()
            .map_err(|error| invalid(error.to_string()))?
            .flatten();
        object
            .finish()
            .map_err(|error| invalid(error.to_string()))?;
        let Some(name) = name else {
            return Err(invalid("it needs interface.".into()));
        };
        Interface::from_name(&name)
            .ok_or_else(|| invalid(format!("interface must be retail or modern, not {name:?}.")))
    }

    /// Writes `interface`.
    pub fn save(&self, interface: Interface) -> Result<(), InterfaceSettingsError> {
        let document = ObjectBuilder::new()
            .member("interface", Out::str(interface.name()))
            .build();
        fsutil::write_via_tmp(
            &self.file_path,
            json::to_pretty(&document).as_bytes(),
            false,
        )
        .map_err(|error| InterfaceSettingsError(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(name: &str) -> (PathBuf, LauncherInterfaceSettingsStore) {
        let folder = std::env::temp_dir().join(format!(
            "openac-interface-settings-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let store = LauncherInterfaceSettingsStore::new(&folder.join(INTERFACE_SETTINGS_FILE_NAME));
        (folder, store)
    }

    #[test]
    fn a_missing_file_is_retail() {
        let (folder, store) = store("missing");
        assert_eq!(store.load().unwrap(), Interface::Retail);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_saved_interface_reads_back() {
        let (folder, store) = store("saved");
        store.save(Interface::Modern).unwrap();
        assert_eq!(store.load().unwrap(), Interface::Modern);
        store.save(Interface::Retail).unwrap();
        assert_eq!(store.load().unwrap(), Interface::Retail);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn an_unknown_interface_is_an_error() {
        let (folder, store) = store("unknown");
        std::fs::write(store.file_path(), br#"{"interface": "shiny"}"#).unwrap();
        let error = store.load().unwrap_err();
        assert!(error.0.contains("retail or modern"), "{error}");
        let _ = std::fs::remove_dir_all(folder);
    }
}
