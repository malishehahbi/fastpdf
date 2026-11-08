# Default text font

Place an open-licensed `.ttf` here (e.g. **Roboto-Regular.ttf** or **DejaVuSans.ttf**) so WASM builds embed real glyphs.

On Windows/macOS/Linux, `build.rs` can fall back to a system font when nothing is vendored here.

Override with: `FASTPDF_FONT=/path/to/font.ttf`
