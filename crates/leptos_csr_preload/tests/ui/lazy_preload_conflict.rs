use leptos_csr_preload::lazy;

#[lazy(preload, preload_path = "/inventory")]
async fn load_inventory_seed() -> Vec<String> {
    vec!["adapter".to_string()]
}

fn main() {}
