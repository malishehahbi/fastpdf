use fastpdf_core::{FastPdfDocument, Rect, RenderOutput};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(js_name = version)]
pub fn js_version() -> String {
    FastPdfDocument::version().to_string()
}

fn to_js_err(e: fastpdf_core::FastPdfError) -> JsError {
    JsError::new(&e.to_string())
}

#[wasm_bindgen]
pub struct WasmDocument {
    inner: FastPdfDocument,
}

#[wasm_bindgen]
pub struct WasmRenderResult {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[wasm_bindgen]
impl WasmRenderResult {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[wasm_bindgen(js_name = pixels)]
    pub fn pixels_js(&self) -> Uint8Array {
        Uint8Array::from(self.pixels.as_slice())
    }
}

impl From<RenderOutput> for WasmRenderResult {
    fn from(r: RenderOutput) -> Self {
        Self {
            width: r.width,
            height: r.height,
            pixels: r.pixels,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    #[wasm_bindgen(js_name = openBytes)]
    pub fn open_bytes(data: &[u8]) -> Result<WasmDocument, JsError> {
        let inner = FastPdfDocument::from_bytes(data).map_err(to_js_err)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = openBytesWithPassword)]
    pub fn open_bytes_with_password(data: &[u8], password: &str) -> Result<WasmDocument, JsError> {
        let inner = FastPdfDocument::from_bytes_with_password(data, password).map_err(to_js_err)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = pageCount)]
    pub fn page_count(&self) -> u32 {
        self.inner.page_count()
    }

    #[wasm_bindgen(js_name = getOutline)]
    pub fn get_outline(&self) -> Result<JsValue, JsError> {
        let outline = self.inner.outline().map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&outline).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getInfo)]
    pub fn get_info(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.inner.info()).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getPermissions)]
    pub fn get_permissions(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.inner.permissions())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = isEncrypted)]
    pub fn is_encrypted(&self) -> bool {
        self.inner.is_encrypted()
    }

    #[wasm_bindgen(js_name = getPageInfo)]
    pub fn get_page_info(&self, index: u32) -> Result<JsValue, JsError> {
        let info = self.inner.page_info(index).map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = renderPage)]
    pub fn render_page(&mut self, index: u32, scale: f32) -> Result<WasmRenderResult, JsError> {
        self.inner
            .render_page(index, scale)
            .map(WasmRenderResult::from)
            .map_err(to_js_err)
    }

    #[wasm_bindgen(js_name = renderThumbnail)]
    pub fn render_thumbnail(&mut self, index: u32, max_side: f32) -> Result<WasmRenderResult, JsError> {
        self.inner
            .render_thumbnail(index, max_side)
            .map(WasmRenderResult::from)
            .map_err(to_js_err)
    }

    #[wasm_bindgen(js_name = renderTile)]
    pub fn render_tile(
        &mut self,
        index: u32,
        scale: f32,
        tile_x: f32,
        tile_y: f32,
        tile_w: f32,
        tile_h: f32,
    ) -> Result<WasmRenderResult, JsError> {
        self.inner
            .render_tile(index, scale, tile_x, tile_y, tile_w, tile_h)
            .map(WasmRenderResult::from)
            .map_err(to_js_err)
    }

    #[wasm_bindgen(js_name = setMaxCacheEntries)]
    pub fn set_max_cache_entries(&mut self, n: usize) {
        self.inner.set_max_cache_entries(n);
    }

    #[wasm_bindgen(js_name = clearCache)]
    pub fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }

    #[wasm_bindgen(js_name = getTextSpans)]
    pub fn get_text_spans(&self, index: u32) -> Result<JsValue, JsError> {
        let spans = self.inner.text_spans(index).map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&spans).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = search)]
    pub fn search(&mut self, query: &str) -> Result<JsValue, JsError> {
        let hits = self.inner.search(query).map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&hits).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getAnnotations)]
    pub fn get_annotations(&self, index: u32) -> Result<JsValue, JsError> {
        let a = self.inner.annotations(index).map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&a).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = hitTestAnnotations)]
    pub fn hit_test_annotations(
        &self,
        index: u32,
        x: f32,
        y: f32,
    ) -> Result<JsValue, JsError> {
        let a = self.inner.hit_test_annotation(index, x, y).map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&a).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getLinks)]
    pub fn get_links(&self, index: u32) -> Result<JsValue, JsError> {
        let links = self.inner.links_on_page(index).map_err(to_js_err)?;
        serde_wasm_bindgen::to_value(&links).map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = getFormFields)]
    pub fn get_form_fields(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.inner.form_fields())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    #[wasm_bindgen(js_name = setFormField)]
    pub fn set_form_field(&mut self, name: &str, value: &str) -> bool {
        self.inner.set_form_field(name, value)
    }

    #[wasm_bindgen(js_name = addHighlight)]
    pub fn add_highlight(
        &mut self,
        page_index: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        contents: Option<String>,
    ) -> Result<(), JsError> {
        let quad = Rect {
            x,
            y,
            width,
            height,
        };
        self.inner
            .add_highlight(page_index, quad, contents.as_deref())
            .map_err(to_js_err)?;
        Ok(())
    }

    #[wasm_bindgen(js_name = saveToBytes)]
    pub fn save_to_bytes(&mut self) -> Result<Uint8Array, JsError> {
        let bytes = self.inner.save_to_bytes().map_err(to_js_err)?;
        Ok(Uint8Array::from(bytes.as_slice()))
    }

    #[wasm_bindgen(js_name = appendBytes)]
    pub fn append_bytes(&mut self, chunk: &[u8]) -> Result<(), JsError> {
        self.inner.append_bytes(chunk).map_err(to_js_err)
    }
}
