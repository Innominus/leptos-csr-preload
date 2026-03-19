use leptos::{prelude::*, task::spawn_local};
use leptos_csr_preload::{lazy, lazy_route};
use leptos_router::{
    components::{FlatRoutes, Route, Router, A},
    hooks::use_params_map,
    Lazy, LazyRoute, OptionalParamSegment, ParamSegment, StaticSegment, WildcardSegment,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main>
                <nav>
                    <A href="/">"Home"</A>
                    <A href="/about">"About"</A>
                    <A href="/info">"Info"</A>
                    <A href="/inventory">"Inventory"</A>
                    <A href="/reports">"Reports"</A>
                    <A href="/reports/2026">"Reports 2026"</A>
                    <A href="/files/assets/icons/logo.svg">"Files"</A>
                    <A href="/users/alice">"User Alice"</A>
                </nav>

                <FlatRoutes fallback=|| view! { <p id="not-found">"Not found."</p> }>
                    <Route path=StaticSegment("") view=Home />
                    <Route path=StaticSegment("about") view={Lazy::<AboutRoute>::new()} />
                    <Route path=StaticSegment("info") view={Lazy::<AboutRoute>::new()} />
                    <Route path=StaticSegment("inventory") view={Lazy::<InventoryRoute>::new()} />
                    <Route path=(StaticSegment("reports"), OptionalParamSegment("id")) view={Lazy::<ReportsRoute>::new()} />
                    <Route path=(StaticSegment("files"), WildcardSegment("rest")) view={Lazy::<FilesRoute>::new()} />
                    <Route path=(StaticSegment("users"), ParamSegment("id")) view={Lazy::<UserRoute>::new()} />
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
fn Home() -> impl IntoView {
    let message = RwSignal::new("Not loaded yet.".to_string());
    let load_global = move |_| {
        message.set("Loading global badge...".to_string());
        spawn_local(async move {
            message.set(load_global_badge().await);
        });
    };

    view! {
        <section>
            <h1 id="page-title">"Home"</h1>
            <p id="home-copy">"This page is eagerly loaded."</p>
            <p id="global-badge-message">{move || message.get()}</p>
            <button id="global-badge-load" on:click=load_global>
                "Load global badge"
            </button>
        </section>
    }
}

#[lazy(name = global_badge, preload)]
async fn load_global_badge() -> String {
    "global badge loaded from an always-preloaded lazy function".to_string()
}

#[lazy(preload_path = "/inventory")]
async fn load_inventory_seed() -> Vec<String> {
    vec![
        "adapter".to_string(),
        "ratchet".to_string(),
        "sprocket".to_string(),
    ]
}

#[lazy]
async fn load_inventory_refresh() -> String {
    "inventory refreshed from a named lazy function".to_string()
}

#[lazy(preload_path = "/reports/:id?")]
async fn load_reports_summary() -> String {
    "reports summary loaded from a preloaded lazy function".to_string()
}

#[lazy(preload_path = "/files/*rest")]
async fn load_file_preview() -> String {
    "file preview loaded from a preloaded lazy function".to_string()
}

#[lazy(name = shared_banner, preload_paths = ["/reports/:id?", "/files/*rest"])]
async fn load_shared_banner() -> String {
    "shared banner loaded from a multi-path lazy function".to_string()
}

#[lazy(preload_path = "/users/:id")]
async fn load_user_metrics() -> String {
    "user metrics loaded during initial route render".to_string()
}

#[lazy]
async fn load_user_badges() -> String {
    "user badges loaded after interaction".to_string()
}

#[lazy_route(preload_paths = ["/about", "/info"])]
impl LazyRoute for AboutRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! {
            <section>
                <h1 id="page-title">"About"</h1>
                <p id="about-copy">"This is a lazy route with only route-level preloads."</p>
            </section>
        }
        .into_any()
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct AboutRoute;

#[allow(dead_code)]
#[derive(Clone)]
pub struct InventoryRoute {
    items: LocalResource<Vec<String>>,
}

#[lazy_route(preload_path = "/inventory")]
impl LazyRoute for InventoryRoute {
    fn data() -> Self {
        Self {
            items: LocalResource::new(load_inventory_seed),
        }
    }

    fn view(this: Self) -> AnyView {
        let message = RwSignal::new("Not refreshed yet.".to_string());
        let refresh = move |_| {
            message.set("Refreshing inventory...".to_string());
            spawn_local(async move {
                message.set(load_inventory_refresh().await);
            });
        };

        let items = move || Suspend::new(async move { this.items.await.join(", ") });

        view! {
            <section>
                <h1 id="page-title">"Inventory"</h1>
                <p id="inventory-items">
                    <Suspense fallback=|| view! { <span>"Loading seed..."</span> }>
                        {items}
                    </Suspense>
                </p>
                <p id="inventory-message">{move || message.get()}</p>
                <button id="inventory-refresh" on:click=refresh>
                    "Refresh inventory"
                </button>
            </section>
        }
        .into_any()
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ReportsRoute {
    summary: LocalResource<String>,
    banner: LocalResource<String>,
}

#[lazy_route(preload_path = "/reports/:id?")]
impl LazyRoute for ReportsRoute {
    fn data() -> Self {
        Self {
            summary: LocalResource::new(load_reports_summary),
            banner: LocalResource::new(load_shared_banner),
        }
    }

    fn view(this: Self) -> AnyView {
        let params = use_params_map();
        let summary = move || Suspend::new(async move { this.summary.await });
        let banner = move || Suspend::new(async move { this.banner.await });

        view! {
            <section>
                <h1 id="page-title">"Reports"</h1>
                <p id="reports-copy">
                    {move || format!(
                        "Showing report {}",
                        params.get().get("id").unwrap_or_else(|| "latest".to_string())
                    )}
                </p>
                <p id="reports-summary">
                    <Suspense fallback=|| view! { <span>"Loading reports summary..."</span> }>
                        {summary}
                    </Suspense>
                </p>
                <p id="reports-banner">
                    <Suspense fallback=|| view! { <span>"Loading reports banner..."</span> }>
                        {banner}
                    </Suspense>
                </p>
            </section>
        }
        .into_any()
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct FilesRoute {
    preview: LocalResource<String>,
    banner: LocalResource<String>,
}

#[lazy_route(preload_path = "/files/*rest")]
impl LazyRoute for FilesRoute {
    fn data() -> Self {
        Self {
            preview: LocalResource::new(load_file_preview),
            banner: LocalResource::new(load_shared_banner),
        }
    }

    fn view(this: Self) -> AnyView {
        let params = use_params_map();
        let preview = move || Suspend::new(async move { this.preview.await });
        let banner = move || Suspend::new(async move { this.banner.await });

        view! {
            <section>
                <h1 id="page-title">"Files"</h1>
                <p id="files-copy">
                    {move || format!(
                        "Inspecting path {}",
                        params.get().get("rest").unwrap_or_default()
                    )}
                </p>
                <p id="files-preview">
                    <Suspense fallback=|| view! { <span>"Loading file preview..."</span> }>
                        {preview}
                    </Suspense>
                </p>
                <p id="files-banner">
                    <Suspense fallback=|| view! { <span>"Loading file banner..."</span> }>
                        {banner}
                    </Suspense>
                </p>
            </section>
        }
        .into_any()
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct UserRoute {
    metrics: LocalResource<String>,
}

#[lazy_route(preload_path = "/users/:id")]
impl LazyRoute for UserRoute {
    fn data() -> Self {
        Self {
            metrics: LocalResource::new(load_user_metrics),
        }
    }

    fn view(this: Self) -> AnyView {
        let params = use_params_map();
        let message = RwSignal::new("Not loaded yet.".to_string());
        let load_badges = move |_| {
            message.set("Loading badges...".to_string());
            spawn_local(async move {
                message.set(load_user_badges().await);
            });
        };
        let metrics = move || Suspend::new(async move { this.metrics.await });

        view! {
            <section>
                <h1 id="page-title">"User"</h1>
                <p id="user-copy">
                    {move || format!("Viewing user {}", params.get().get("id").unwrap_or_default())}
                </p>
                <p id="user-metrics">
                    <Suspense fallback=|| view! { <span>"Loading metrics..."</span> }>
                        {metrics}
                    </Suspense>
                </p>
                <p id="user-message">{move || message.get()}</p>
                <button id="user-badges" on:click=load_badges>
                    "Load user badges"
                </button>
            </section>
        }
        .into_any()
    }
}

pub fn preload_registry() -> leptos_csr_preload::PreloadRegistry {
    leptos_csr_preload::collect_preload_registry()
}
