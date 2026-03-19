use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreloadError {
    #[error("error reading split manifest file `{path}`")]
    ReadManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("error parsing split manifest file `{path}`")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("error reading index.html file `{path}`")]
    ReadIndexHtml {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("error reading dist directory `{path}`")]
    ReadDistDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("no split manifest was found in `{path}`")]
    MissingManifestFile { path: PathBuf },
    #[error("multiple split manifests were found in `{path}`")]
    MultipleManifestFiles { path: PathBuf },
    #[error("invalid route pattern `{pattern}`: {reason}")]
    InvalidRoutePattern {
        pattern: String,
        reason: &'static str,
    },
    #[error("preload `{source_name}` expected a manifest key starting with `{prefix}`")]
    MissingManifestKey { prefix: String, source_name: String },
    #[error("preload `{source_name}` prefix `{prefix}` resolved to multiple keys: {matches:?}")]
    AmbiguousManifestKey {
        prefix: String,
        source_name: String,
        matches: Vec<String>,
    },
}
