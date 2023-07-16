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

fn main() {
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



}


