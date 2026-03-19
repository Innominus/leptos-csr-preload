use leptos::prelude::*;
use leptos_csr_preload::lazy_route;
use leptos_router::LazyRoute;

#[derive(Clone)]
struct InvalidArgumentRoute;

#[lazy_route(path = "/invalid")]
impl LazyRoute for InvalidArgumentRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <p>"invalid arg"</p> }.into_any()
    }
}

fn main() {}
