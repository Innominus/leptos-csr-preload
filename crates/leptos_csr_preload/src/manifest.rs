use crate::PreloadError;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrunkWasmSplitManifest {
    pub loader: String,
    pub prefetch_map: BTreeMap<String, Vec<String>>,
}

impl TrunkWasmSplitManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, PreloadError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| PreloadError::ReadManifest {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_slice(&bytes).map_err(|source| PreloadError::ParseManifest {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn from_dist_dir(dist_dir: impl AsRef<Path>) -> Result<Self, PreloadError> {
        let manifest_path = find_manifest_path(dist_dir.as_ref())?;
        Self::from_path(manifest_path)
    }
}

pub(crate) fn find_manifest_path(dist_dir: &Path) -> Result<PathBuf, PreloadError> {
    let mut manifests = fs::read_dir(dist_dir)
        .map_err(|source| PreloadError::ReadDistDir {
            path: dist_dir.to_path_buf(),
            source,
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("__wasm_split_manifest") && name.ends_with(".json")
                })
        })
        .collect::<Vec<_>>();

    manifests.sort();
    match manifests.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(PreloadError::MissingManifestFile {
            path: dist_dir.to_path_buf(),
        }),
        _ => Err(PreloadError::MultipleManifestFiles {
            path: dist_dir.to_path_buf(),
        }),
    }
}
