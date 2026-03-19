#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::Router;
    use leptos_csr_preload::axum::SpaFallbackBuilder;

    let preload_registry = axum_csr_preload_app::preload_registry();
    let dist_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/dist");
    let app = Router::new().fallback_service(
        SpaFallbackBuilder::new(&dist_dir, preload_registry)
            .precompressed_gzip()
            .precompressed_br()
            .build()?,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    println!("serving CSR app with route preloads at http://127.0.0.1:3001");
    axum::serve(listener, app).await?;
    Ok(())
}
