use std::env;
use std::path::Path;

use pdfium_rs::Document;

fn print_usage() {
    eprintln!("usage: pdfium-rs render <input.pdf> <output.png> [--dpi DPI]");
    eprintln!();
    eprintln!("options:");
    eprintln!("  --dpi DPI    Render at the specified DPI (default: 72)");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];
    if command != "render" {
        eprintln!("error: unknown command '{}', expected 'render'", command);
        print_usage();
        std::process::exit(1);
    }

    let input_pdf = &args[2];
    let output_png = &args[3];
    let mut dpi = 72.0;

    // Parse optional --dpi argument
    let mut i = 4;
    while i < args.len() {
        if args[i] == "--dpi" {
            if i + 1 >= args.len() {
                eprintln!("error: --dpi requires a value");
                std::process::exit(1);
            }
            match args[i + 1].parse::<f32>() {
                Ok(d) if d > 0.0 && d.is_finite() => dpi = d,
                _ => {
                    eprintln!(
                        "error: --dpi value must be a positive number, got '{}'",
                        args[i + 1]
                    );
                    std::process::exit(1);
                }
            }
            i += 2;
        } else {
            eprintln!("error: unknown option '{}'", args[i]);
            print_usage();
            std::process::exit(1);
        }
    }

    // Load PDF
    let mut doc = match Document::open(Path::new(input_pdf)) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: failed to load PDF: {}", e);
            std::process::exit(1);
        }
    };

    // Get first page
    let page = match doc.page(0) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: failed to get page 0: {}", e);
            std::process::exit(1);
        }
    };

    // Render
    let bitmap = match page.render(dpi) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: render failed: {}", e);
            std::process::exit(1);
        }
    };

    // Encode PNG
    let png_data = match bitmap.encode_png() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: PNG encoding failed: {}", e);
            std::process::exit(1);
        }
    };

    // Write PNG to file
    if let Err(e) = std::fs::write(output_png, png_data) {
        eprintln!("error: failed to write PNG file: {}", e);
        std::process::exit(1);
    }

    println!("✓ rendered {} to {} at {} DPI", input_pdf, output_png, dpi);
}
