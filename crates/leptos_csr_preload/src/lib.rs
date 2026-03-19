#![doc = include_str!("../README.md")]

mod error;
mod html;
mod index;
mod manifest;
mod pattern;
mod registry;

#[cfg(feature = "axum")]
pub mod axum;

pub use error::PreloadError;
pub use html::{asset_href, render_html_with_preloads, HtmlResponseBackend, IndexHtmlTemplate};
pub use index::{PreloadIndex, PreloadIndexEntry, ResolvedPreloads};
pub use leptos_csr_preload_macros::{lazy, lazy_route};
pub use manifest::TrunkWasmSplitManifest;
pub use pattern::RoutePattern;
pub use registry::{
    collect_preload_registry, PreloadRegistration, PreloadRegistry, RegisteredPreload,
};

pub mod __private {
    pub use inventory;
}

#[cfg(test)]
mod tests;
