use crate::{PreloadError, PreloadRegistry, RoutePattern, TrunkWasmSplitManifest};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPreloads {
    pub loader: String,
    pub split_keys: Vec<String>,
    pub wasm_chunks: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PreloadIndexEntry {
    pub source_name: Arc<str>,
    pub pattern: RoutePattern,
    pub specificity: (usize, usize, usize, usize),
    pub split_prefix: Arc<str>,
    pub split_key: Arc<str>,
    pub wasm_chunks: Arc<[String]>,
}

#[derive(Debug, Clone)]
pub struct PreloadIndex {
    loader: Arc<str>,
    global_entries: Vec<PreloadIndexEntry>,
    entries: Vec<PreloadIndexEntry>,
}

impl PreloadIndex {
    pub fn from_registry(
        manifest: TrunkWasmSplitManifest,
        registry: &PreloadRegistry,
    ) -> Result<Self, PreloadError> {
        let resolved_entries = registry
            .iter()
            .map(|entry| {
                let resolved =
                    resolve_manifest_entry(&manifest, &entry.split_prefix, &entry.source_name)?;
                let split_key = resolved
                    .split_keys
                    .first()
                    .cloned()
                    .expect("resolved preload entry must contain exactly one split key");

                let pattern = match &entry.preload_path {
                    Some(path) => Some(RoutePattern::parse(path)?),
                    None => None,
                };
                let specificity = pattern.as_ref().map(RoutePattern::specificity).unwrap_or((
                    usize::MAX,
                    usize::MAX,
                    usize::MAX,
                    usize::MAX,
                ));
                let entry_pattern = pattern.clone().unwrap_or_else(|| {
                    RoutePattern::parse("/").expect("root route pattern is valid")
                });

                Ok((
                    pattern,
                    PreloadIndexEntry {
                        source_name: Arc::from(entry.source_name.as_str()),
                        specificity,
                        pattern: entry_pattern,
                        split_prefix: Arc::from(entry.split_prefix.as_str()),
                        split_key: Arc::from(split_key),
                        wasm_chunks: Arc::from(resolved.wasm_chunks),
                    },
                ))
            })
            .collect::<Result<Vec<_>, PreloadError>>()?;

        let mut global_entries = Vec::new();
        let mut entries = Vec::new();
        for (pattern, entry) in resolved_entries {
            if pattern.is_some() {
                entries.push(entry);
            } else {
                global_entries.push(entry);
            }
        }

        entries.sort_by(|left, right| right.specificity.cmp(&left.specificity));
        global_entries.sort_by(|left, right| left.source_name.cmp(&right.source_name));

        Ok(Self {
            loader: Arc::from(manifest.loader),
            global_entries,
            entries,
        })
    }

    pub fn from_dist_dir(
        dist_dir: impl AsRef<std::path::Path>,
        registry: &PreloadRegistry,
    ) -> Result<Self, PreloadError> {
        let manifest = TrunkWasmSplitManifest::from_dist_dir(dist_dir)?;
        Self::from_registry(manifest, registry)
    }

    pub fn resolve(&self, path: &str) -> Option<ResolvedPreloads> {
        let mut best_specificity = None;
        let mut matched = self.global_entries.iter().collect::<Vec<_>>();

        for entry in &self.entries {
            if !entry.pattern.matches(path) {
                continue;
            }

            match best_specificity {
                None => {
                    best_specificity = Some(entry.specificity);
                    matched.push(entry);
                }
                Some(current) if entry.specificity > current => {
                    best_specificity = Some(entry.specificity);
                    matched.clear();
                    matched.push(entry);
                }
                Some(current) if entry.specificity == current => matched.push(entry),
                Some(_) => {}
            }
        }

        if matched.is_empty() {
            return None;
        }

        let mut split_keys = matched
            .iter()
            .map(|entry| entry.split_key.to_string())
            .collect::<Vec<_>>();
        split_keys.sort();

        let mut wasm_chunks = BTreeSet::new();
        for entry in matched {
            for chunk in entry.wasm_chunks.iter() {
                wasm_chunks.insert(chunk.clone());
            }
        }

        Some(ResolvedPreloads {
            loader: self.loader.to_string(),
            split_keys,
            wasm_chunks: wasm_chunks.into_iter().collect(),
        })
    }

    pub fn has_path_match(&self, path: &str) -> bool {
        self.entries.iter().any(|entry| entry.pattern.matches(path))
    }

    pub fn entries(&self) -> &[PreloadIndexEntry] {
        &self.entries
    }

    pub fn global_entries(&self) -> &[PreloadIndexEntry] {
        &self.global_entries
    }
}

pub(crate) fn resolve_manifest_entry(
    manifest: &TrunkWasmSplitManifest,
    prefix: &str,
    source_name: &str,
) -> Result<ResolvedPreloads, PreloadError> {
    let matches = manifest
        .prefetch_map
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(PreloadError::MissingManifestKey {
            prefix: prefix.to_string(),
            source_name: source_name.to_string(),
        }),
        [(key, files)] => Ok(ResolvedPreloads {
            loader: manifest.loader.clone(),
            split_keys: vec![(*key).clone()],
            wasm_chunks: (*files).clone(),
        }),
        _ => Err(PreloadError::AmbiguousManifestKey {
            prefix: prefix.to_string(),
            source_name: source_name.to_string(),
            matches: matches.iter().map(|(key, _)| (*key).clone()).collect(),
        }),
    }
}
