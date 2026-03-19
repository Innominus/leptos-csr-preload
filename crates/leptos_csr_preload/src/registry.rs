#[derive(Debug)]
pub struct RegisteredPreload {
    pub source_name: &'static str,
    pub preload_path: Option<&'static str>,
    pub split_prefix: &'static str,
}

inventory::collect!(RegisteredPreload);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreloadRegistration {
    pub source_name: String,
    pub preload_path: Option<String>,
    pub split_prefix: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreloadRegistry {
    entries: Vec<PreloadRegistration>,
}

impl PreloadRegistry {
    pub fn new(entries: Vec<PreloadRegistration>) -> Self {
        Self { entries }
    }

    pub fn iter(&self) -> impl Iterator<Item = &PreloadRegistration> {
        self.entries.iter()
    }
}

pub fn collect_preload_registry() -> PreloadRegistry {
    let entries = inventory::iter::<RegisteredPreload>
        .into_iter()
        .map(|entry| PreloadRegistration {
            source_name: entry.source_name.to_string(),
            preload_path: entry.preload_path.map(str::to_string),
            split_prefix: entry.split_prefix.to_string(),
        })
        .collect();

    PreloadRegistry::new(entries)
}
