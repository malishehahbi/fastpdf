use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Returns the FastPDF engine version (from `fastpdf-core`).
#[wasm_bindgen(js_name = version)]
pub fn js_version() -> String {
    fastpdf_core::VERSION.to_string()
}
