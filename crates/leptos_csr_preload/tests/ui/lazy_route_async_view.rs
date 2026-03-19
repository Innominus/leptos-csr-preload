use leptos::prelude::*;
use leptos_csr_preload::lazy_route;
use leptos_router::LazyRoute;

#[derive(Clone)]
struct AsyncViewRoute;

#[lazy_route(preload_path = "/async")]
impl LazyRoute for AsyncViewRoute {
    fn data() -> Self {
        Self
    }

    async fn view(_this: Self) -> AnyView {
        view! { <p>"async view"</p> }.into_any()
    }
}

fn main() {}
