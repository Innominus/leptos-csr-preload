#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(axum_csr_preload_app::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("build this crate with trunk to produce the CSR assets");
}
