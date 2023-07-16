use clap::Parser;
use std::path::PathBuf;
use pdfium_render::prelude::*;


// Compare two pdf documents

// Define and collect arguments
#[derive(Parser, Debug)]
#[command(name = "match_pdf")]
#[command(author = "author")]
#[command(version = "0.1")]
#[command(about = "Compares two pdf documents.", long_about = None)]
struct Cli {
    original_pdf1_path: PathBuf,
    original_pdf2_path: PathBuf,

    /// An optional 'stop' flag: Stop the comparison after the first page where differences are found.
    #[arg(short, long)]
    stop: bool,

    /// An optional 'page' flag: Stop the comparison after # pages if a difference was found in the first # pages.
    #[arg(short, long)]
    page: Option<i32>,

}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    println!("pdf1: {}", cli.original_pdf1_path.display());
    println!("pdf2: {}", cli.original_pdf2_path.display());

    if cli.stop {
        println!("The 'stop' flag was set.  The comparison will stop after the first page with differences.");
    } else {
        println!("The 'stop' flag was not set.");
    }

    match cli.page {
        Some(value) => println!("The 'page' flag was set with the value {}.", value),
        None => println!("The 'page' flag was not set."),
    }

    // Bind to the pdfium library (pdfium.dll)
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")).unwrap()
    );

    // Load the document from the given path...
    let pdf_document_1 = pdfium.load_pdf_from_file(&cli.original_pdf1_path, None)?;

    // ... set rendering options that will be applied to all pages...

    let render_config = PdfRenderConfig::new()
    .set_target_width(2000)
    .set_maximum_height(2000)
    .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

    // ... then render each page to a bitmap image, saving each image to a JPEG file.

    for (index, page) in pdf_document_1.pages().iter().enumerate() {
        page.render_with_config(&render_config)?
            .as_image() // Renders this page to an image::DynamicImage...
            .as_rgba8() // ... then converts it to an image::Image...
            .ok_or(PdfiumError::ImageError)?
            .save_with_format(
                format!("test-page-{}.jpg", index), 
                image::ImageFormat::Jpeg
            ) // ... and saves it to a file.
            .map_err(|_| PdfiumError::ImageError)?;
    }


    Ok(())

}


