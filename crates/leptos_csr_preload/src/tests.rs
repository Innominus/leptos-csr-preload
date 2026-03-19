use crate::{
    asset_href, IndexHtmlTemplate, PreloadError, PreloadIndex, PreloadRegistration,
    PreloadRegistry, RoutePattern, TrunkWasmSplitManifest,
};
use std::collections::BTreeMap;

fn manifest() -> TrunkWasmSplitManifest {
    TrunkWasmSplitManifest {
        loader: "__wasm_split-hash.js".to_string(),
        prefetch_map: BTreeMap::from([
            (
                "global_badge_999".to_string(),
                vec!["split-global-badge.wasm".to_string()],
            ),
            (
                "about_route_123".to_string(),
                vec!["split-about.wasm".to_string()],
            ),
            (
                "shared_banner_777".to_string(),
                vec!["split-shared-banner.wasm".to_string()],
            ),
            (
                "inventory_route_111".to_string(),
                vec![
                    "chunk-a.wasm".to_string(),
                    "split-inventory.wasm".to_string(),
                ],
            ),
            (
                "load_inventory_seed_222".to_string(),
                vec!["chunk-b.wasm".to_string(), "split-seed.wasm".to_string()],
            ),
            (
                "user_route_333".to_string(),
                vec!["chunk-user.wasm".to_string(), "split-user.wasm".to_string()],
            ),
            (
                "load_user_metrics_444".to_string(),
                vec!["split-metrics.wasm".to_string()],
            ),
            (
                "reports_route_555".to_string(),
                vec!["split-reports.wasm".to_string()],
            ),
            (
                "files_route_666".to_string(),
                vec!["split-files.wasm".to_string()],
            ),
        ]),
    }
}

fn registry() -> PreloadRegistry {
    PreloadRegistry::new(vec![
        PreloadRegistration {
            source_name: "global_badge".to_string(),
            preload_path: None,
            split_prefix: "global_badge_".to_string(),
        },
        PreloadRegistration {
            source_name: "about_route".to_string(),
            preload_path: Some("/about".to_string()),
            split_prefix: "about_route_".to_string(),
        },
        PreloadRegistration {
            source_name: "about_route".to_string(),
            preload_path: Some("/info".to_string()),
            split_prefix: "about_route_".to_string(),
        },
        PreloadRegistration {
            source_name: "inventory_route".to_string(),
            preload_path: Some("/inventory".to_string()),
            split_prefix: "inventory_route_".to_string(),
        },
        PreloadRegistration {
            source_name: "load_inventory_seed".to_string(),
            preload_path: Some("/inventory".to_string()),
            split_prefix: "load_inventory_seed_".to_string(),
        },
        PreloadRegistration {
            source_name: "user_route".to_string(),
            preload_path: Some("/users/:id".to_string()),
            split_prefix: "user_route_".to_string(),
        },
        PreloadRegistration {
            source_name: "load_user_metrics".to_string(),
            preload_path: Some("/users/:id".to_string()),
            split_prefix: "load_user_metrics_".to_string(),
        },
        PreloadRegistration {
            source_name: "reports_route".to_string(),
            preload_path: Some("/reports/:id?".to_string()),
            split_prefix: "reports_route_".to_string(),
        },
        PreloadRegistration {
            source_name: "files_route".to_string(),
            preload_path: Some("/files/*rest".to_string()),
            split_prefix: "files_route_".to_string(),
        },
        PreloadRegistration {
            source_name: "shared_banner".to_string(),
            preload_path: Some("/reports/:id?".to_string()),
            split_prefix: "shared_banner_".to_string(),
        },
        PreloadRegistration {
            source_name: "shared_banner".to_string(),
            preload_path: Some("/files/*rest".to_string()),
            split_prefix: "shared_banner_".to_string(),
        },
    ])
}

#[test]
fn route_pattern_matches_static_parameterized_optional_splat_and_query_paths() {
    let about = RoutePattern::parse("/about").unwrap();
    let users = RoutePattern::parse("/users/:id").unwrap();
    let reports = RoutePattern::parse("/reports/:id?").unwrap();
    let files = RoutePattern::parse("/files/*rest").unwrap();

    assert!(about.matches("/about"));
    assert!(users.matches("/users/alice"));
    assert!(users.matches("/users/alice?tab=overview"));
    assert!(reports.matches("/reports"));
    assert!(reports.matches("/reports/42"));
    assert!(files.matches("/files/a/b/c"));
    assert!(!users.matches("/users"));
}

#[test]
fn route_pattern_rejects_invalid_patterns() {
    let err = RoutePattern::parse("/users/:").unwrap_err();
    assert!(matches!(err, PreloadError::InvalidRoutePattern { .. }));

    let err = RoutePattern::parse("/files/*").unwrap_err();
    assert!(matches!(err, PreloadError::InvalidRoutePattern { .. }));
}

#[test]
fn preload_index_unions_matching_route_and_function_preloads() {
    let index = PreloadIndex::from_registry(manifest(), &registry()).unwrap();

    let resolved = index.resolve("/inventory").unwrap();

    assert_eq!(
        resolved.split_keys,
        vec![
            "global_badge_999".to_string(),
            "inventory_route_111".to_string(),
            "load_inventory_seed_222".to_string(),
        ]
    );
    assert!(resolved
        .wasm_chunks
        .contains(&"split-global-badge.wasm".to_string()));
    assert!(resolved
        .wasm_chunks
        .contains(&"split-inventory.wasm".to_string()));
    assert!(resolved
        .wasm_chunks
        .contains(&"split-seed.wasm".to_string()));
}

#[test]
fn preload_index_matches_parameterized_query_optional_and_splat_paths() {
    let index = PreloadIndex::from_registry(manifest(), &registry()).unwrap();

    let user = index.resolve("/users/alice?tab=profile").unwrap();
    assert!(user.split_keys.contains(&"global_badge_999".to_string()));
    assert!(user.split_keys.contains(&"user_route_333".to_string()));
    assert!(user
        .split_keys
        .contains(&"load_user_metrics_444".to_string()));

    let reports = index.resolve("/reports").unwrap();
    assert_eq!(
        reports.split_keys,
        vec![
            "global_badge_999".to_string(),
            "reports_route_555".to_string(),
            "shared_banner_777".to_string()
        ]
    );

    let files = index.resolve("/files/images/icons/logo.svg").unwrap();
    assert_eq!(
        files.split_keys,
        vec![
            "files_route_666".to_string(),
            "global_badge_999".to_string(),
            "shared_banner_777".to_string()
        ]
    );
}

#[test]
fn preload_index_supports_multiple_preload_paths_for_the_same_split_point() {
    let index = PreloadIndex::from_registry(manifest(), &registry()).unwrap();

    let about = index.resolve("/about").unwrap();
    let info = index.resolve("/info").unwrap();

    assert_eq!(
        about.split_keys,
        vec![
            "about_route_123".to_string(),
            "global_badge_999".to_string()
        ]
    );
    assert_eq!(
        info.split_keys,
        vec![
            "about_route_123".to_string(),
            "global_badge_999".to_string()
        ]
    );
}

#[test]
fn preload_index_includes_global_preloads_on_all_routes() {
    let index = PreloadIndex::from_registry(manifest(), &registry()).unwrap();

    let about = index.resolve("/about").unwrap();
    let unknown = index.resolve("/totally-unknown-route").unwrap();

    assert!(about.split_keys.contains(&"global_badge_999".to_string()));
    assert_eq!(unknown.split_keys, vec!["global_badge_999".to_string()]);
}

#[test]
fn preload_index_reports_missing_prefixes() {
    let missing = PreloadRegistry::new(vec![PreloadRegistration {
        source_name: "missing_route".to_string(),
        preload_path: Some("/missing".to_string()),
        split_prefix: "missing_route_".to_string(),
    }]);

    let err = PreloadIndex::from_registry(manifest(), &missing).unwrap_err();
    assert!(matches!(err, PreloadError::MissingManifestKey { .. }));
}

#[test]
fn template_injects_new_preloads_without_duplicate_hrefs() {
    let template = IndexHtmlTemplate::parse(
        r#"<html><head><link rel="modulepreload" href="/__wasm_split-hash.js"></head><body></body></html>"#,
    );
    let preloads = crate::ResolvedPreloads {
        loader: "__wasm_split-hash.js".to_string(),
        split_keys: vec!["about_route_123".to_string()],
        wasm_chunks: vec!["split-about.wasm".to_string()],
    };

    let html = template.render("/", Some(&preloads));

    assert!(html.contains("/split-about.wasm"));
    assert_eq!(html.matches("/__wasm_split-hash.js").count(), 1);
    assert_eq!(asset_href("/", "foo.wasm"), "/foo.wasm");
}
