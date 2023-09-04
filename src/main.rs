// main

mod common;
mod images;

use clap::Parser;
use image::DynamicImage;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::process;
use std::path::PathBuf;
use std::path::Path;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};


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

    /// An optional 'justdiff' flag: In combination with 'output', only different pages are included in output file.
    #[arg(short, long)]
    justdiff: bool,

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


// Define the structure that will be used for excluded rectangles if a config file is specified
#[derive(Debug, Deserialize, Serialize)]
pub struct Rectangle {
    pub page: String,
    pub top_left: [i32; 2],
    pub bottom_right: [i32; 2],
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub ignored_rectangles: Vec<Rectangle>,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Define global variables
    let mut differences_found_in_document: bool = false;
    let mut differences_found_in_page: bool;
    let mut differences_in_number_of_pages: bool = false;


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
    
        if cli.justdiff {
            println!("The 'justdiff' flag was set.  Only different pages will be included in the output file.");
        } else {
            println!("The 'justdiff' flag was not set.");
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
            Some(ref value) => println!("The 'config' flag was set with value:  {}", value.to_string_lossy()),
            None => println!("The 'config' flag was not set."),
        }
    
    } 
    

    // If the user provided an output folder, check it
    if let Some(ref _value) = cli.output {

        if let Some(ref path) = cli.output {
            
            // If the output path provided does not exist, exit now.
            if ! Path::new(path).exists() {
                println!("The provided output folder does not exist.");
                process::exit(1);
            } 
        } 
    }


    // If the config argument was used, evaluate and prep the data
    if let Some(ref _value) = cli.config {

        if let Some(ref path) = cli.config {
            
            // If the config path does not exist, exit now.
            if ! Path::new(path).exists() {
                println!("The provided config file does not exist.");
                process::exit(1);
            } 

            // Consume the contents of the json file, placing them into the previously defined JSON object
            // Read the file to a string
            let mut file = File::open(path).expect("Failed to open the specified config file.");
            let mut content = String::new();
            file.read_to_string(&mut content).expect("Failed to read the specified config file.");

            // Deserialize the JSON content to the Config struct
            let config: Config = serde_json::from_str(&content).expect("Failed to deserialize JSON within the specified config file.");
            
            println!("{:?}", config);

        } 
    }


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


    // Get the number of pages for each document
    let doc1_pages = pdf_document_1.pages().len();
    let doc2_pages = pdf_document_2.pages().len();


    // If the number of pages is different, we're done
    if doc1_pages != doc2_pages {
        differences_in_number_of_pages = true;

        if cli.debug {
            println!("The number of pages in the documents is different.");
        }
    }

    // ... set pdf to image rendering options that will be applied to all pages...
    let render_config = PdfRenderConfig::new()
    .set_target_width(2000)
    .set_maximum_height(2000)
    .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true)
    .render_form_data(false);


    // Create a variable to hold the PDF document if it's needed
    let mut output_pdf = pdfium.create_new_pdf().unwrap();


    // If the number of pages in the two documents are the same, proceed with the comparison
    if ! differences_in_number_of_pages {

        // ... then iterate through the pages until reaching the end of the shortest document
        for index in 0..doc1_pages {

            // Reset page differences variable
            differences_found_in_page = false;

            if cli.debug {
                if index % 10 == 0 {
                    println!("Processing page {:?}", (index + 1));
                }
            }

            // Create the objects for each of the pages to be compared
            let doc1page = pdf_document_1.pages().get(index)?;
            let doc2page = pdf_document_2.pages().get(index)?;

            // Get the dimensions of the pages
            let doc1width = doc1page.width();
            let doc1height = doc1page.height();
            let doc2width = doc2page.width();
            let doc2height = doc2page.height();



            // If the size of the pages id different
            if (doc1width != doc2width) ||
                (doc1height != doc2height) {

                if cli.debug {
                    println!("Page {:?} of the two docs are different sizes.  Ending the comparison.", (index + 1));
                }
                
                differences_found_in_document = true;

                // Break out of the for loop and end this.  Comparing pixels of pages that are different sizes ends badly.
                break;
            }


            // Create an image of the current page from document 1
            let image1 = images::render_page(&doc1page, &render_config)?;

            // Create an image of the current page from document 2
            let image2 = images::render_page(&doc2page, &render_config)?;




            // Compare the images of the two pages
            let page_differences_vector = images::compare_images_in_chunks(&image1, &image2);

            // Set the differences_found variables to true if the vector is not empty
            if !page_differences_vector.is_empty(){
                differences_found_in_document = true;
                differences_found_in_page = true;

                if cli.debug {
                    println!("page_differences_vector for page {:?}: {:?}", index, page_differences_vector);
                }

            }


            /******************************************************
            If a results file is desired, highlight the differences in the images, and add to a results file
            ******************************************************/

            // If the user used the 'output' argument 
            if let Some(ref _value) = cli.output{
            
                // AND (there are page differences OR justdiff is false)
                if differences_found_in_page || !cli.justdiff {

                    // Take actions to highlight differences and create an output document

                    // Create the highlighted image variables in the current scope
                    let doc1_page_highlighted_image;
                    let doc2_page_highlighted_image;

                    // Highlight the differences within the images
                    // If differences were found in page
                    if differences_found_in_page {

                        doc1_page_highlighted_image = images::highlight_chunks(&image1, &page_differences_vector);

                        doc2_page_highlighted_image = images::highlight_chunks(&image2, &page_differences_vector);
                    
                    } else { // Else there are not differences in this page, so just use the non-highlighted images

                        doc1_page_highlighted_image = image1;

                        doc2_page_highlighted_image = image2;
                    }

                    // PDF documents use points as a unit of measurement, and there are 72 points to an inch.
                    const POINTS_PER_INCH: f32 = 72.0;

                    // Desired page size in inches
                    let desired_width_in_inches = 17.0;  // For example: Letter width
                    // let desired_height_in_inches = 11.0;  // For example: Letter height

                    // Calculate the desired size in points
                    let desired_width_in_points = desired_width_in_inches * POINTS_PER_INCH;
                    // let desired_height_in_points = desired_height_in_inches * POINTS_PER_INCH;

                    // Calculate the scaling factor based on the desired width
                    let scale_factor = desired_width_in_points / (doc1_page_highlighted_image.width() + doc2_page_highlighted_image.width()) as f32;

                    // Apply the scaling factor to image sizes and positions
                    let width = (doc1_page_highlighted_image.width() + doc2_page_highlighted_image.width()) as f32 * scale_factor;
                    let height = doc1_page_highlighted_image.height() as f32 * scale_factor;

                    let paper_size = PdfPagePaperSize::Custom(PdfPoints::new(width), PdfPoints::new(height));

                    let image1_x_position_in_points = PdfPoints::new(0.0);
                    let image1_y_position_in_points = PdfPoints::new(0.0);
                    let image2_x_position_in_points = PdfPoints::new(doc1_page_highlighted_image.width() as f32 * scale_factor);
                    let image2_y_position_in_points = PdfPoints::new(0.0);


                    // Add a page to the output pdf document
                    let page = output_pdf
                        .pages_mut()
                        .create_page_at_end(paper_size);

                    // Check to see if the page is a page, since it was actually wrapped in a result enum
                    if let Ok(mut page) = page {
                        // Add the image to the page
                        
                        // Document 1
                        // Convert the image from document 1 into the type that is acceptable for writing to the page
                        let dynamic_image = DynamicImage::ImageRgba8(doc1_page_highlighted_image.clone());
                        let image1_width = doc1_page_highlighted_image.width() as f32 * scale_factor;

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
                        let image2_width = doc2_page_highlighted_image.width() as f32 * scale_factor;

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
                        println!("Something went wrong when adding a page to the output PDF document.");
                    }

                }

            }        

            /******************************************************
            If stop is true and differences have been found, stop the comparison.
            ******************************************************/
            if cli.stop && differences_found_in_document{
                
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

            

        } // End of the for loop iterating through each page

    }

    // Clean up, the comparison is over.

    // If the user used the 'output' argument, write the output PDF file to the specified location.
    if let Some(ref _value) = cli.output {

        if let Some(ref path) = cli.output {
            output_pdf.save_to_file(path)?;
        } else {
            println!("There is an issue with the file path provided as the output.");
        }

    }


    if differences_found_in_document || differences_in_number_of_pages {

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
