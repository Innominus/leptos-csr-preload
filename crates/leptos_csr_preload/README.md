# leptos_csr_preload

`leptos_csr_preload` is a small helper crate for pure CSR Leptos apps built with Trunk's
split-WASM support.

It lets an app export a preload registry from the same Rust source that defines its lazy routes and
lazy functions, then lets a custom backend resolve that registry through Trunk's emitted
`__wasm_split_manifest-*.json` file and inject route-specific preload tags into `index.html`.

## Core idea

- Trunk emits the generic split manifest: `split key -> hashed wasm files`
- This crate emits preload metadata: `path pattern -> split prefix`
- A backend combines the two and injects preload tags into HTML responses

Static hosts can ignore the preload registry entirely. Custom backends can use it to avoid
waterfalls on deep links.

The current preload resolver is designed around flat route matching. For nested route trees, the
practical recommendation today is to attach preload metadata to the parent route that should cover
the nested subtree.

## Main APIs

- `#[lazy(...)]`
  - wraps `#[leptos::lazy]`
  - supports optional `name = ...`
  - supports optional `preload` to always preload the split on every HTML response
  - supports optional `preload_path = "..."` for backend preloading
  - supports optional `preload_paths = ["...", ...]` to preload the same split point on multiple routes
- `#[lazy_route(...)]`
  - wraps a Leptos `LazyRoute` impl
  - supports optional `name = ...`
  - supports optional `preload_path = "..."`
  - supports optional `preload_paths = ["...", ...]`
- `collect_preload_registry()`
  - returns the exported registry for native backends
  - note: the consuming app crate must define and enable a `preload-registry` feature when it is
    built as a native dependency of the server, because the proc macros emit registry entries behind
    `#[cfg(feature = "preload-registry")]`
- `PreloadIndex`
  - resolves path matches through Trunk's manifest
- `axum` feature
  - provides a simple Axum SPA fallback handler with HTML preload injection
  - includes `SpaFallbackBuilder` for enabling precompressed static asset variants

## Example

```rust
use leptos::{prelude::*, task::spawn_local};
use leptos_csr_preload::{lazy, lazy_route};
use leptos_router::{Lazy, LazyRoute};

#[lazy(name = global_badge, preload)]
async fn load_global_badge() -> String {
    "global badge".to_string()
}

#[lazy(preload_path = "/inventory")]
async fn load_inventory_seed() -> Vec<String> {
    vec!["adapter".to_string(), "ratchet".to_string()]
}

#[lazy(name = shared_banner, preload_paths = ["/inventory", "/reports/:id?"])]
async fn load_shared_banner() -> String {
    "shared banner".to_string()
}

#[derive(Clone)]
struct InventoryRoute {
    items: LocalResource<Vec<String>>,
}

#[lazy_route(preload_paths = ["/inventory", "/stockroom"])]
impl LazyRoute for InventoryRoute {
    fn data() -> Self {
        Self {
            items: LocalResource::new(load_inventory_seed),
        }
    }

    fn view(this: Self) -> AnyView {
        let items = move || Suspend::new(async move { this.items.await.join(", ") });
        view! {
            <section>
                <Suspense fallback=|| view! { <span>"Loading..."</span> }>
                    {items}
                </Suspense>
            </section>
        }
        .into_any()
    }
}

pub fn preload_registry() -> leptos_csr_preload::PreloadRegistry {
    leptos_csr_preload::collect_preload_registry()
}
```

## Route pattern syntax

- static: `/about`
- parameterized: `/users/:id`
- optional parameter: `/reports/:id?`
- splat: `/files/*rest`

Query strings are ignored during path matching.
