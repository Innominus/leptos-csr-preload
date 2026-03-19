use crate::{
    render_html_with_preloads, HtmlResponseBackend, IndexHtmlTemplate, PreloadError, PreloadIndex,
    PreloadRegistry,
};
use axum::{
    body::Body,
    extract::{Request as AxumRequest, State},
    http::{header, Request, Response, StatusCode},
    response::{Html, IntoResponse, Response as AxumResponse},
    routing::{any, MethodRouter},
    Router,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tower::util::ServiceExt;
use tower_http::services::ServeDir;

#[derive(Debug, Clone, Copy, Default)]
struct PrecompressedAssets {
    gzip: bool,
    br: bool,
    deflate: bool,
    zstd: bool,
}

#[derive(Clone)]
pub struct AxumPreloadState {
    pub dist_dir: Arc<PathBuf>,
    pub static_service: Arc<ServeDir>,
    pub template: Arc<IndexHtmlTemplate>,
    pub preload_index: Arc<PreloadIndex>,
    pub asset_base: Arc<String>,
}

impl AxumPreloadState {
    pub fn from_dist_dir(
        dist_dir: impl Into<PathBuf>,
        registry: &PreloadRegistry,
    ) -> Result<Self, PreloadError> {
        Self::from_dist_dir_with_options(
            dist_dir,
            registry,
            "/".to_string(),
            PrecompressedAssets::default(),
        )
    }

    fn from_dist_dir_with_options(
        dist_dir: impl Into<PathBuf>,
        registry: &PreloadRegistry,
        asset_base: String,
        precompressed: PrecompressedAssets,
    ) -> Result<Self, PreloadError> {
        let dist_dir = dist_dir.into();
        let index_path = dist_dir.join("index.html");
        let index_html =
            std::fs::read_to_string(&index_path).map_err(|source| PreloadError::ReadIndexHtml {
                path: index_path,
                source,
            })?;
        let template = IndexHtmlTemplate::parse(index_html);
        let preload_index = PreloadIndex::from_dist_dir(&dist_dir, registry)?;

        Ok(Self {
            static_service: Arc::new(build_static_service(&dist_dir, precompressed)),
            dist_dir: Arc::new(dist_dir),
            template: Arc::new(template),
            preload_index: Arc::new(preload_index),
            asset_base: Arc::new(asset_base),
        })
    }

    pub fn new(
        dist_dir: impl Into<PathBuf>,
        static_service: ServeDir,
        template: IndexHtmlTemplate,
        preload_index: PreloadIndex,
    ) -> Self {
        Self {
            dist_dir: Arc::new(dist_dir.into()),
            static_service: Arc::new(static_service),
            template: Arc::new(template),
            preload_index: Arc::new(preload_index),
            asset_base: Arc::new("/".to_string()),
        }
    }

    pub fn with_asset_base(mut self, asset_base: impl Into<String>) -> Self {
        self.asset_base = Arc::new(asset_base.into());
        self
    }
}

#[derive(Clone, Copy)]
pub struct AxumBackend;

impl HtmlResponseBackend for AxumBackend {
    type Response = AxumResponse;

    fn respond_html(&self, html: String) -> Self::Response {
        Html(html).into_response()
    }
}

#[derive(Debug, Clone)]
pub struct SpaFallbackBuilder {
    dist_dir: PathBuf,
    registry: PreloadRegistry,
    asset_base: String,
    precompressed: PrecompressedAssets,
}

impl SpaFallbackBuilder {
    pub fn new(dist_dir: impl Into<PathBuf>, registry: PreloadRegistry) -> Self {
        Self {
            dist_dir: dist_dir.into(),
            registry,
            asset_base: "/".to_string(),
            precompressed: PrecompressedAssets::default(),
        }
    }

    pub fn asset_base(mut self, asset_base: impl Into<String>) -> Self {
        self.asset_base = asset_base.into();
        self
    }

    pub fn precompressed_gzip(mut self) -> Self {
        self.precompressed.gzip = true;
        self
    }

    pub fn precompressed_br(mut self) -> Self {
        self.precompressed.br = true;
        self
    }

    pub fn precompressed_deflate(mut self) -> Self {
        self.precompressed.deflate = true;
        self
    }

    pub fn precompressed_zstd(mut self) -> Self {
        self.precompressed.zstd = true;
        self
    }

    pub fn build_state(self) -> Result<AxumPreloadState, PreloadError> {
        AxumPreloadState::from_dist_dir_with_options(
            self.dist_dir,
            &self.registry,
            self.asset_base,
            self.precompressed,
        )
    }

    pub fn build(self) -> Result<MethodRouter, PreloadError> {
        let state = self.build_state()?;

        Ok(any(move |request: AxumRequest| {
            let state = state.clone();
            async move { serve_request(request, state).await }
        }))
    }

    pub fn build_router(self) -> Result<Router, PreloadError> {
        Ok(Router::new().fallback_service(self.build()?))
    }
}

pub fn spa_fallback(
    dist_dir: impl Into<PathBuf>,
    registry: PreloadRegistry,
) -> Result<MethodRouter, PreloadError> {
    SpaFallbackBuilder::new(dist_dir, registry).build()
}

pub fn router_with_spa_fallback(
    dist_dir: impl Into<PathBuf>,
    registry: PreloadRegistry,
) -> Result<Router, PreloadError> {
    SpaFallbackBuilder::new(dist_dir, registry).build_router()
}

pub async fn file_or_index_handler(
    State(state): State<AxumPreloadState>,
    request: AxumRequest,
) -> AxumResponse {
    serve_request(request, state).await
}

async fn serve_request(request: AxumRequest, state: AxumPreloadState) -> AxumResponse {
    let uri = request.uri().clone();
    let has_path_match = state.preload_index.has_path_match(uri.path());
    let preloads = state.preload_index.resolve(uri.path());
    let should_fallback =
        request_accepts_html(&request) && (has_path_match || path_looks_like_spa_route(uri.path()));

    match get_static_file(request, state.static_service.as_ref()).await {
        Ok(response) if response.status() == StatusCode::OK => return response,
        Ok(response) if !should_fallback => {
            return response;
        }
        Ok(_) => {}
        Err(err) => return err.into_response(),
    }

    render_html_with_preloads(
        &AxumBackend,
        &state.template,
        &state.asset_base,
        preloads.as_ref(),
    )
}

fn path_looks_like_spa_route(path: &str) -> bool {
    let normalized = path.split('?').next().unwrap_or(path).trim_end_matches('/');
    if normalized.is_empty() {
        return true;
    }

    let last_segment = normalized.rsplit('/').next().unwrap_or(normalized);
    !last_segment.contains('.')
}

fn request_accepts_html(request: &AxumRequest) -> bool {
    if !matches!(request.method().as_str(), "GET" | "HEAD") {
        return false;
    }

    let Some(accept) = request.headers().get(header::ACCEPT) else {
        return true;
    };
    let Ok(accept) = accept.to_str() else {
        return false;
    };

    accept.contains("text/html") || accept.contains("application/xhtml+xml")
}

async fn get_static_file(
    request: Request<Body>,
    static_service: &ServeDir,
) -> Result<Response<Body>, (StatusCode, String)> {
    match static_service.clone().oneshot(request).await {
        Ok(response) => Ok(response.into_response()),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("error serving static file: {err}"),
        )),
    }
}

fn build_static_service(root: &Path, precompressed: PrecompressedAssets) -> ServeDir {
    let mut service = ServeDir::new(root);
    if precompressed.gzip {
        service = service.precompressed_gzip();
    }
    if precompressed.br {
        service = service.precompressed_br();
    }
    if precompressed.deflate {
        service = service.precompressed_deflate();
    }
    if precompressed.zstd {
        service = service.precompressed_zstd();
    }
    service
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        IndexHtmlTemplate, PreloadIndex, PreloadRegistration, PreloadRegistry,
        TrunkWasmSplitManifest,
    };
    use axum::{
        body::Body,
        http::{header, HeaderValue, Request},
        Router,
    };
    use std::{collections::BTreeMap, fs};
    use tempfile::tempdir;
    use tower::util::ServiceExt;

    fn manifest() -> TrunkWasmSplitManifest {
        TrunkWasmSplitManifest {
            loader: "__wasm_split-hash.js".to_string(),
            prefetch_map: BTreeMap::from([
                (
                    "global_badge_111".to_string(),
                    vec!["split-global-badge.wasm".to_string()],
                ),
                (
                    "user_route_123".to_string(),
                    vec!["chunk-user.wasm".to_string(), "split-user.wasm".to_string()],
                ),
                (
                    "load_user_metrics_456".to_string(),
                    vec!["split-metrics.wasm".to_string()],
                ),
                (
                    "reports_route_789".to_string(),
                    vec!["split-reports.wasm".to_string()],
                ),
                (
                    "files_route_987".to_string(),
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
        ])
    }

    fn state(temp: &Path) -> AxumPreloadState {
        let template = IndexHtmlTemplate::parse(
            "<html><head><link rel=\"modulepreload\" href=\"/__wasm_split-hash.js\"></head><body></body></html>",
        );
        let index = PreloadIndex::from_registry(manifest(), &registry()).unwrap();
        AxumPreloadState::new(
            temp,
            build_static_service(temp, PrecompressedAssets::default()),
            template,
            index,
        )
    }

    #[tokio::test]
    async fn asset_404s_do_not_fallback_to_index_html() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        let app = Router::new()
            .fallback(file_or_index_handler)
            .with_state(state(temp.path()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/missing.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn route_requests_inject_matching_route_and_function_preloads() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        let app = Router::new()
            .fallback(file_or_index_handler)
            .with_state(state(temp.path()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users/alice?tab=settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("split-global-badge.wasm"));
        assert!(body.contains("split-user.wasm"));
        assert!(body.contains("split-metrics.wasm"));
    }

    #[tokio::test]
    async fn non_html_accept_header_keeps_non_navigation_requests_out_of_spa_fallback() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        let app = Router::new()
            .fallback(file_or_index_handler)
            .with_state(state(temp.path()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users/alice")
                    .header(header::ACCEPT, HeaderValue::from_static("application/json"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn non_get_methods_do_not_fallback_to_html() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        let app = Router::new()
            .fallback(file_or_index_handler)
            .with_state(state(temp.path()));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/alice")
                    .header(header::ACCEPT, HeaderValue::from_static("text/html"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn helper_builds_a_fallback_service() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        fs::write(
            temp.path().join("__wasm_split_manifest-hash.json"),
            serde_json::to_vec(&manifest()).unwrap(),
        )
        .unwrap();
        let app = Router::new().fallback_service(spa_fallback(temp.path(), registry()).unwrap());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users/alice?tab=settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("split-global-badge.wasm"));
        assert!(body.contains("split-user.wasm"));
        assert!(body.contains("split-metrics.wasm"));
    }

    #[tokio::test]
    async fn optional_parameter_routes_inject_preloads() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        let app = Router::new()
            .fallback(file_or_index_handler)
            .with_state(state(temp.path()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/reports/2026?tab=trend")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("split-global-badge.wasm"));
        assert!(body.contains("split-reports.wasm"));
    }

    #[tokio::test]
    async fn splat_routes_inject_preloads() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        let app = Router::new()
            .fallback(file_or_index_handler)
            .with_state(state(temp.path()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/files/assets/icons/logo.svg")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("split-global-badge.wasm"));
        assert!(body.contains("split-files.wasm"));
    }

    #[tokio::test]
    async fn gzip_precompressed_assets_are_served_when_enabled() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("app.js"), "plain-js").unwrap();
        fs::write(temp.path().join("app.js.gz"), "gzip-js").unwrap();
        let service = build_static_service(
            temp.path(),
            PrecompressedAssets {
                gzip: true,
                ..Default::default()
            },
        );

        let req = Request::builder()
            .uri("/app.js")
            .header(
                header::ACCEPT_ENCODING,
                HeaderValue::from_static("gzip, br"),
            )
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(req).await.unwrap().into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_ENCODING),
            Some(&HeaderValue::from_static("gzip"))
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(body, "gzip-js");
    }

    #[tokio::test]
    async fn brotli_precompressed_assets_are_served_when_enabled() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("app.js"), "plain-js").unwrap();
        fs::write(temp.path().join("app.js.br"), "brotli-js").unwrap();
        let service = build_static_service(
            temp.path(),
            PrecompressedAssets {
                br: true,
                ..Default::default()
            },
        );

        let req = Request::builder()
            .uri("/app.js")
            .header(
                header::ACCEPT_ENCODING,
                HeaderValue::from_static("br, gzip"),
            )
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(req).await.unwrap().into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_ENCODING),
            Some(&HeaderValue::from_static("br"))
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(body, "brotli-js");
    }

    #[tokio::test]
    async fn plain_assets_are_served_when_precompressed_variants_are_disabled() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("app.js"), "plain-js").unwrap();
        fs::write(temp.path().join("app.js.gz"), "gzip-js").unwrap();
        let service = build_static_service(temp.path(), PrecompressedAssets::default());

        let req = Request::builder()
            .uri("/app.js")
            .header(header::ACCEPT_ENCODING, HeaderValue::from_static("gzip"))
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(req).await.unwrap().into_response();
        assert!(response.headers().get(header::CONTENT_ENCODING).is_none());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(body, "plain-js");
    }

    #[tokio::test]
    async fn helper_builder_supports_precompressed_assets_and_spa_fallback() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("index.html"), "ignored").unwrap();
        fs::write(
            temp.path().join("__wasm_split_manifest-hash.json"),
            serde_json::to_vec(&manifest()).unwrap(),
        )
        .unwrap();
        fs::write(temp.path().join("app.js"), "plain-js").unwrap();
        fs::write(temp.path().join("app.js.gz"), "gzip-js").unwrap();

        let app = Router::new().fallback_service(
            SpaFallbackBuilder::new(temp.path(), registry())
                .precompressed_gzip()
                .build()
                .unwrap(),
        );

        let asset_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/app.js")
                    .header(header::ACCEPT_ENCODING, HeaderValue::from_static("gzip"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            asset_response.headers().get(header::CONTENT_ENCODING),
            Some(&HeaderValue::from_static("gzip"))
        );
        let asset_body = axum::body::to_bytes(asset_response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(asset_body, "gzip-js");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users/alice?tab=settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("split-user.wasm"));
        assert!(body.contains("split-metrics.wasm"));
    }
}
