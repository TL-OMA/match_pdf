// main

mod common;
mod images;

use clap::Parser;
use image::{ImageBuffer, DynamicImage};
use std::fs;
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


    /// An optional 'debug' flag: Include verbose output to the console.
    #[arg(short, long)]
    debug: bool,

    /// An optional 'stop' flag: Stop the comparison after the first page where differences are found.
    #[arg(short, long)]
    stop: bool,

    /// An optional 'pages' flag: Stop the comparison after ## pages if a difference was found in the first ## pages.
    #[arg(short, long)]
    pages: Option<i32>,

    /// An optional 'maxpages' flag: At a maximum, compare ## pages.  Can be combined with other flags.
    #[arg(short, long)]
    maxpages: Option<i32>,

    /// An optional 'output' flag: Use with a file path to indicate where to place a results file.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// An optional 'config' flag: Use with a file path to indicate where to find the config file.
    #[arg(short, long)]
    config: Option<PathBuf>,

}



fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Define global variables
    let mut differences_found: bool = false;
    let mut output_is_set: bool = false;


    // Parse the command line arguments

    let cli = Cli::parse();

    println!("pdf1: {}", cli.original_pdf1_path.display());
    println!("pdf2: {}", cli.original_pdf2_path.display());

    
    // If the debug flag is set, print some flag and argument messages to the console
    if cli.debug {
        println!("The 'debug' flag was set.  More information will be provided at the console.");
    
        if cli.stop {
            println!("The 'stop' flag was set.  The comparison will stop after the first page with differences.");
        } else {
            println!("The 'stop' flag was not set.");
        }
    
        match cli.pages {
            Some(value) => println!("The 'pages' flag was set with value:  {}", value),
            None => println!("The 'pages' flag was not set."),
        }
    
        match cli.maxpages {
            Some(value) => println!("The 'maxpages' flag was set with value:  {}", value),
            None => println!("The 'maxpages' flag was not set."),
        }
    
        match cli.output {
            Some(ref value) => println!("The 'output' flag was set with value:  {}", value.to_string_lossy()),
            None => println!("The 'output' flag was not set."),
        }
    
        match cli.config {
            Some(value) => println!("The 'config' flag was set with value:  {}", value.to_string_lossy()),
            None => println!("The 'config' flag was not set."),
        }
    
    } 
    

    // If the output argument is true, set the global output_is_set var
    match cli.output {
        Some(ref value) => output_is_set = true,
        _ => (), // Do nothing when None occurs.
    }


    // If the config argument was used, evaluate and prep the data




    // Define a temp folder to use based on the system temp folder

    let temp_path: PathBuf = common::get_temp_dir("pdf_match");

    if cli.debug {
        println!("App-specific temp directory is: {:?}", temp_path);
    }

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
    .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true)
    .render_form_data(false);

    // Create a variable to hold the PDF document if it's needed
    let mut output_pdf = pdfium.create_new_pdf().unwrap();

    // ... then iterate through the pages until reaching the end of the shortest document
    for index in 0..min_pages {

        if index % 10 == 0 {
            println!("Processing page {:?}", index);
        }

        // Create an image of the current page from document 1
        let doc1page = pdf_document_1.pages().get(index)?;
        let image1 = images::render_page(&doc1page, &render_config)?;

        // Create an image of the current page from document 2
        let doc2page = pdf_document_2.pages().get(index)?;
        let image2 = images::render_page(&doc2page, &render_config)?;


        // Compare the images of the two pages
        let page_differences_vector = images::compare_images_in_chunks(&image1, &image2);

        // Set the DIFFERENCES_FOUND variable to true if the vector is not empty
        if !page_differences_vector.is_empty(){
            differences_found = true;

            if cli.debug {
                println!("page_differences_vector for page {:?}: {:?}", index, page_differences_vector);
            }

        }


        /******************************************************
        If a results file is desired, highlight the differences in the images, and add to a results file
        ******************************************************/

        // If the user used the 'output' argument
        if let Some(ref value) = cli.output {
          
            // Take actions to highlight differences and create an output document

            // Highlight the differences within the images
            let doc1_page_highlighted_image = images::highlight_chunks(&image1, &page_differences_vector);

            let doc2_page_highlighted_image = images::highlight_chunks(&image2, &page_differences_vector);

            
            // If debug is turned on, write these image files to disk
            if cli.debug {

                // Create a unique path/filename and write to disk for debugging purposes
                let mut image_path3 = temp_path.clone();
                image_path3.push(format!("doc1-page-{}-highlighted.jpg", index));

                doc1_page_highlighted_image.save_with_format( 
                    image_path3,
                    image::ImageFormat::Jpeg
                ) // ... and saves it to a file.
                .map_err(|_| PdfiumError::ImageError)?;

                // Create a unique path/filename and write to disk for debugging purposes
                let mut image_path4 = temp_path.clone();
                image_path4.push(format!("doc2-page-{}-highlighted.jpg", index));

                doc2_page_highlighted_image.save_with_format( 
                    image_path4,
                    image::ImageFormat::Jpeg
                ) // ... and saves it to a file.
                .map_err(|_| PdfiumError::ImageError)?;
            }


            // Create a size for the page that is about to be added
            let width = doc1_page_highlighted_image.width() + doc2_page_highlighted_image.width();
            let height = doc1_page_highlighted_image.height();

            let width_in_points = PdfPoints::new(width as f32);
            let height_in_points = PdfPoints::new(height as f32);

            let paper_size = PdfPagePaperSize::Custom(width_in_points, height_in_points);

            let image1_x_position_in_points = PdfPoints::new(0 as f32);
            let image1_y_position_in_points = PdfPoints::new(0 as f32);
            let image2_x_position_in_points = PdfPoints::new(doc1_page_highlighted_image.width() as f32);
            let image2_y_position_in_points = PdfPoints::new(0 as f32);


            // Add a page to the output pdf document
            let mut page = output_pdf
                .pages_mut()
                .create_page_at_end(paper_size);

            // Check to see if the page is a page, since it was actually wrapped in a result enum
            if let Ok(mut page) = page {
                // Add the image to the page
                //add_image_to_pdf_page(&pdfium, &mut output_pdf, &mut page, &doc1_page_highlighted_image, 0, 0);
                
                // Document 1
                // Convert the image from document 1 into the type that is acceptable for writing to the page
                let dynamic_image = DynamicImage::ImageRgba8(doc1_page_highlighted_image.clone());
                let image1_width = doc1_page_highlighted_image.width().clone();

                // Make a PDF document object using the image from document 1
                let mut object = PdfPageImageObject::new_with_width(
                    &output_pdf,
                    &dynamic_image,
                    PdfPoints::new(image1_width as f32),
                )?;
            
                // Describe the placement of the object
                object.translate(image1_x_position_in_points, image1_y_position_in_points)?;
            
                // Add the image from document 1 to the destination PDF page.
                page.objects_mut().add_image_object(object)?;

                // Document 2
                // Convert the image from document 2 into the type that is acceptable for writing to the page
                let dynamic_image2 = DynamicImage::ImageRgba8(doc2_page_highlighted_image.clone());
                let image2_width = doc2_page_highlighted_image.width().clone();

                // Make a PDF document object using the image from document 2
                let mut object2 = PdfPageImageObject::new_with_width(
                    &output_pdf,
                    &dynamic_image2,
                    PdfPoints::new(image2_width as f32),
                )?;
            
                // Describe the placement of the object - put this one on the right side
                object2.translate(image2_x_position_in_points, image2_y_position_in_points)?;
            
                // Add the image from document 2 to the destination PDF page.
                page.objects_mut().add_image_object(object2)?;
            
            } else {
                // Handle the error case
                eprintln!("Error when getting the PDF page");
            }

            //let mut doc_path = temp_path.clone();
            //doc_path.push(format!("output.pdf"));
            
            output_pdf.save_to_file("c:\\temp\\output\\myTestPDF.pdf")?;
        }        

        /******************************************************
        If stop is true and differences have been found, stop the comparison.
        ******************************************************/
        if cli.stop && differences_found{
            
            // Break out of the for loop and finish up
            break;
        }
        

        /******************************************************
        If the current page is equal to the 'pages' value, stop if there have been differences.
        ******************************************************/
        // If the user used the pages flag and gave it a value
        if let Some(value) = cli.pages {

            // If the index (page) is the same as the page the user specified
            if value == (index + 1) as i32{

                if cli.debug {
                    println!("The 'pages' flag was set with value {}, so the comparison is stopping now since differences were found.", value);
                }

                // Break out of the for loop and finish up
                break;
            }

        }

        /******************************************************
        If the current page is the maxpages value, stop the comparison.
        ******************************************************/
        // If the user used the max pages flag and gave it a value
        if let Some(value) = cli.maxpages {

            // If the index (page) is the same as the page the user specified
            if value == (index + 1) as i32{

                if cli.debug {
                    println!("The 'maxpages' flag was set with value {value}, so the comparison is stopping now.");
                }

                // Break out of the for loop and finish up
                break;
            }

        }

        

    } // End of the for loop iterating through each page of the the shortest pdf document


    // Clean up, the comparison is over.

    if differences_found {

        println!("Differences were found.")
        
    } else {

        println!("The PDF documents match.")

    }
    

    // Remove the temp folder if not in debug mode.
    if !cli.debug {
        fs::remove_dir_all(temp_path)?;
    } else {
        println!("Since the debug flag is set, the app-specific temp directory was not removed: {:?}", temp_path);
    }


    Ok(())

}


// fn add_image_to_pdf_page(
//     pdfium: &Pdfium,
//     output_pdf: &mut PdfDocument,
//     page: &mut PdfPage,
//     image: &ImageBuffer<image::Rgba<u8>, Vec<u8>>,
//     x: i32,
//     y: i32,
// ) {
//     let image_width = image.width();
//     let image_height = image.height();

//     let mut object = PdfPageImageObject::new_with_width(
//         &output_pdf,
//         &image,
//         PdfPoints::new(image_width),
//     )?;




//     // let image_stream = PdfStream::new(pdfium);
//     // image_stream.write_image(image);

//     // let image_object = PdfImageObject::new(pdfium, image_stream);

//     // page.add_image(image_object, x, y, image_width, image_height);
// }