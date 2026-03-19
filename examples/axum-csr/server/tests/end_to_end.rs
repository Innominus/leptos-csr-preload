use axum::body::Body;
use axum::http::Request;
use axum::Router;
use leptos_csr_preload::axum::spa_fallback;
use leptos_csr_preload::TrunkWasmSplitManifest;
use std::path::{Path, PathBuf};
use std::process::Command;
use tower::util::ServiceExt;

fn app_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../app")
}

fn trunk_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../trunk/Cargo.toml")
}

fn build_example_app() {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(trunk_manifest())
        .arg("--")
        .arg("build")
        .arg("--release")
        .current_dir(app_dir())
        .output()
        .expect("failed to execute trunk build for example app");

    assert!(
        output.status.success(),
        "trunk build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn response_body(path: &str, app: &Router) -> (u16, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("test request should build"),
        )
        .await
        .expect("router should respond");
    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn builds_manifest_and_injects_expected_preloads() {
    build_example_app();

    let dist_dir = app_dir().join("dist");
    let manifest = TrunkWasmSplitManifest::from_dist_dir(&dist_dir).unwrap();
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("global_badge_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("about_route_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("shared_banner_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("inventory_route_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("load_inventory_seed_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("reports_route_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("load_reports_summary_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("files_route_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("load_file_preview_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("user_route_")));
    assert!(manifest
        .prefetch_map
        .keys()
        .any(|key| key.starts_with("load_user_metrics_")));

    let preload_registry = axum_csr_preload_app::preload_registry();
    let app = Router::new().fallback_service(spa_fallback(&dist_dir, preload_registry).unwrap());

    let (status, about) = response_body("/about", &app).await;
    assert_eq!(status, 200);
    assert!(about.contains("split_global_badge_"));
    assert!(about.contains("split_about_route_"));

    let (status, info) = response_body("/info", &app).await;
    assert_eq!(status, 200);
    assert!(info.contains("split_global_badge_"));
    assert!(info.contains("split_about_route_"));

    let (status, inventory) = response_body("/inventory", &app).await;
    assert_eq!(status, 200);
    assert!(inventory.contains("split_global_badge_"));
    assert!(inventory.contains("split_inventory_route_"));
    assert!(inventory.contains("split_load_inventory_seed_"));
    assert!(!inventory.contains("split_load_inventory_refresh_"));

    let (status, reports) = response_body("/reports/2026?tab=trend", &app).await;
    assert_eq!(status, 200);
    assert!(reports.contains("split_global_badge_"));
    assert!(reports.contains("split_reports_route_"));
    assert!(reports.contains("split_load_reports_summary_"));
    assert!(reports.contains("split_shared_banner_"));

    let (status, files) = response_body("/files/assets/icons/logo.svg", &app).await;
    assert_eq!(status, 200);
    assert!(files.contains("split_global_badge_"));
    assert!(files.contains("split_files_route_"));
    assert!(files.contains("split_load_file_preview_"));
    assert!(files.contains("split_shared_banner_"));

    let (status, users) = response_body("/users/alice?tab=settings", &app).await;
    assert_eq!(status, 200);
    assert!(users.contains("split_global_badge_"));
    assert!(users.contains("split_user_route_"));
    assert!(users.contains("split_load_user_metrics_"));
    assert!(!users.contains("split_load_user_badges_"));

    let (status, missing) = response_body("/missing.js", &app).await;
    assert_eq!(status, 404);
    assert!(missing.is_empty());

    let (status, unknown) = response_body("/unknown", &app).await;
    assert_eq!(status, 200);
    assert!(unknown.contains("split_global_badge_"));
}
