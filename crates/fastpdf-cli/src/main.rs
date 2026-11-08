use clap::Parser;
use fastpdf_core::FastPdfDocument;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fastpdf", about = "FastPDF developer CLI")]
struct Args {
  /// Path to PDF file
    path: PathBuf,
  /// Page index to render
    #[arg(short, long, default_value_t = 0)]
    page: u32,
  /// Render scale
    #[arg(short, long, default_value_t = 1.0)]
    scale: f32,
  /// Output PNG path
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let bytes = fs::read(&args.path)?;
    let mut doc = FastPdfDocument::from_bytes(&bytes)?;
    println!("pages: {}", doc.page_count());
    println!("version: {}", FastPdfDocument::version());
    let outline = doc.outline()?;
    println!("outline items: {}", outline.len());
    let rendered = doc.render_page(args.page, args.scale)?;
    if let Some(out) = args.output {
        let img = image::RgbaImage::from_raw(rendered.width, rendered.height, rendered.pixels)
            .ok_or("invalid render buffer")?;
        img.save(out)?;
        println!("saved render");
    }
    Ok(())
}
