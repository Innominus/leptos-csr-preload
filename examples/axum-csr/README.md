# Leptos CSR Preload Example

This example now uses two crates:

- `app/` is the pure CSR Leptos app built by Trunk
- `server/` is an Axum backend that loads the app crate with `preload-registry` enabled and serves
  the built `app/dist/` directory with injected route-specific preload tags

The app crate exports `preload_registry()` explicitly, so the server links and consumes the route
metadata intentionally rather than relying on a no-op anchor function.

The app crate also defines a `preload-registry` feature. The server depends on the app crate with
that feature enabled so the proc-macro-generated registrations are available in the native binary.

The app uses the wrapper macros from `leptos_csr_preload` to register stable route and lazy
function prefixes. Any split point with a `preload_path` participates in backend HTML preloading;
split points without a `preload_path` remain normally lazy. The Axum backend loads Trunk's emitted
split manifest from `app/dist/`, resolves the current request path to the matching route and
function split keys, and injects preload tags into the returned `index.html`.

The app also demonstrates client-side speculative preloading after startup. On the home page,
buttons call the generated `__preload_*` helpers for lazy functions and `LazyRoute::preload()` for
lazy routes before navigation, so you can warm route chunks on demand even without a backend.

The example also demonstrates `preload_paths = [..]`:

- `AboutRoute` is preloaded for both `/about` and `/info`
- `load_shared_banner` is preloaded for both `/reports/:id?` and `/files/*rest`
- `load_global_badge` uses `#[lazy(preload)]`, so it is preloaded on every HTML response

This example uses `FlatRoutes`, which matches the current preload resolver design. For nested route
trees, attach preload metadata to the parent route that should cover the subtree.

Suggested flow:

1. Build the client assets from `app/`: `trunk build --release`
2. Run the Axum server from `server/`: `cargo run`

The server example uses the convenience helper:

```rust
let preload_registry = axum_csr_preload_app::preload_registry();
let dist_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/dist");
let app = Router::new().fallback_service(
    leptos_csr_preload::axum::spa_fallback(&dist_dir, preload_registry)?
);
```

Or, if you want precompressed static assets:

```rust
let app = Router::new().fallback_service(
    leptos_csr_preload::axum::SpaFallbackBuilder::new(&dist_dir, preload_registry)
        .precompressed_gzip()
        .precompressed_br()
        .build()?
);
```

You can still wire things manually with `AxumPreloadState` plus `file_or_index_handler` if you
need more control.

Useful routes:

- `/about`
- `/info`
- `/inventory`
- `/reports`
- `/reports/2026`
- `/files/assets/icons/logo.svg`
- `/users/alice`

Client-side preload example from `app/src/lib.rs`:

```rust
spawn_local(async {
    __preload_load_shared_banner().await;
    __preload_load_reports_summary().await;
    <ReportsRoute as LazyRoute>::preload().await;
});
```

That pattern is useful when you want to preload likely next routes/functions after the first page
load, on hover, or from an explicit user action.
