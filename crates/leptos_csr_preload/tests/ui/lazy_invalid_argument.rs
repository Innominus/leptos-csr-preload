use leptos_csr_preload::lazy;

#[lazy(route = inventory_route)]
async fn load_inventory_seed() -> Vec<String> {
    vec!["adapter".to_string()]
}

fn main() {}
