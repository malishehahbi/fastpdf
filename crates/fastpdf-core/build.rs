use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("default_font.ttf");
    if let Some(src) = locate_font() {
        fs::copy(&src, &dest).expect("copy default font");
        println!("cargo:rerun-if-changed={}", src.display());
    } else {
        panic!(
            "FastPDF needs a TrueType font: place Arial.ttf or Roboto-Regular.ttf in \
             crates/fastpdf-core/assets/fonts/, or set FASTPDF_FONT to a .ttf path"
        );
    }
}

fn locate_font() -> Option<PathBuf> {
    if let Ok(p) = env::var("FASTPDF_FONT") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets = manifest.join("assets/fonts");
    for name in ["Roboto-Regular.ttf", "Arial.ttf", "DejaVuSans.ttf"] {
        let p = assets.join(name);
        if p.is_file() && p.metadata().map(|m| m.len()).unwrap_or(0) > 10_000 {
            return Some(p);
        }
    }
    #[cfg(windows)]
    {
        let arial = PathBuf::from(r"C:\Windows\Fonts\arial.ttf");
        if arial.is_file() {
            return Some(arial);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let arial = PathBuf::from("/Library/Fonts/Arial.ttf");
        if arial.is_file() {
            return Some(arial);
        }
    }
  #[cfg(target_os = "linux")]
    {
        for p in [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ] {
            let path = PathBuf::from(p);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}
