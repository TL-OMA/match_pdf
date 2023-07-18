// main

mod common;
mod images;

use clap::Parser;
use std::path::PathBuf;
use pdfium_render::prelude::*;


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

    // Parse the command line arguments

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


    // Define a temp folder to use based on the system temp folder

    let temp_path: PathBuf = common::get_temp_dir("pdf_match");
    println!("App-specific temp directory is: {:?}", temp_path);


    // Bind to the pdfium library (external, pre-built pdfium.dll)

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")).unwrap()
    );

    // Load the pdf documents...
    let pdf_document_1 = pdfium.load_pdf_from_file(&cli.original_pdf1_path, None)?;
    let pdf_document_2 = pdfium.load_pdf_from_file(&cli.original_pdf2_path, None)?;

    // Get the number of pages for the shortest document
    let min_pages = pdf_document_1.pages().len().min(pdf_document_2.pages().len());

    // ... set pdf to image rendering options that will be applied to all pages...

    let render_config = PdfRenderConfig::new()
    .set_target_width(2000)
    .set_maximum_height(2000)
    .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

    // ... then iterate through each page of the the shortest pdf document

    for index in 0..min_pages {
        let doc1page = pdf_document_1.pages().get(index)?;
        let image1 = images::render_page(&doc1page, &render_config)?;

        let doc2page = pdf_document_2.pages().get(index)?;
        let image2 = images::render_page(&doc2page, &render_config)?;

        // Do something with image1 and image2...

        // Create the path value that includes a unique file name
        let mut image_path1 = temp_path.clone();
        image_path1.push(format!("doc1-page-{}.jpg", index));

        image1.save_with_format( 
            image_path1,
            image::ImageFormat::Jpeg
        ) // ... and saves it to a file.
        .map_err(|_| PdfiumError::ImageError)?;

        // Create the path value that includes a unique file name
        let mut image_path2 = temp_path.clone();
        image_path2.push(format!("doc2-page-{}.jpg", index));

        image2.save_with_format( 
            image_path2,
            image::ImageFormat::Jpeg
        ) // ... and saves it to a file.
        .map_err(|_| PdfiumError::ImageError)?;


        // Compare the images of the two pages
        let page_differences_vector = images::compare_images_in_chunks(&image1, &image2);

        println!("page_differences_vector: {:?}", page_differences_vector);


        // Highlight the differences within the images
        let doc1_page_highlighted_image = images::highlight_chunks(&image1, &page_differences_vector);

        let doc2_page_highlighted_image = images::highlight_chunks(&image2, &page_differences_vector);


        // Create a path value that includes a unique file name
        let mut image_path3 = temp_path.clone();
        image_path3.push(format!("doc1-page-{}-highlighted.jpg", index));

        doc1_page_highlighted_image.save_with_format( 
            image_path3,
            image::ImageFormat::Jpeg
        ) // ... and saves it to a file.
        .map_err(|_| PdfiumError::ImageError)?;

        // Create a path value that includes a unique file name
        let mut image_path4 = temp_path.clone();
        image_path4.push(format!("doc2-page-{}-highlighted.jpg", index));

        doc2_page_highlighted_image.save_with_format( 
            image_path4,
            image::ImageFormat::Jpeg
        ) // ... and saves it to a file.
        .map_err(|_| PdfiumError::ImageError)?;


    }


    Ok(())

}


