use leptos_csr_preload::lazy_route;
use leptos_router::LazyRoute;

#[derive(Clone)]
struct MissingViewRoute;

#[lazy_route(preload_path = "/missing")]
impl LazyRoute for MissingViewRoute {
    fn data() -> Self {
        Self
    }
}

fn main() {}
