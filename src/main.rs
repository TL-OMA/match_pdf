// main

mod common;
mod images;

use clap::Parser;
use image::DynamicImage;
use image::{Rgba, RgbaImage};
use std::fs::{self, File};
use std::io::{self, Read, Write, ErrorKind};
use std::process;
use std::path::PathBuf;
use std::path::Path;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use reqwest;
use reqwest::blocking::Client;
use std::error::Error;


// Define and collect arguments
#[derive(Parser, Debug)]
#[command(name = "match_pdf")]
#[command(author = "author")]
#[command(version = "0.8")]
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

    /// An optional 'result' flag: Use with a file path to indicate where to place a results json file.
    #[arg(short, long)]
    result: Option<PathBuf>,

    /// An optional 'config' flag: Use with a file path to indicate where to find the config file.
    #[arg(short, long)]
    config: Option<PathBuf>,

}


// Define the structure that will be used for excluded rectangles if a config file is specified
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Rectangle {
    pub page: String,
    pub top_left: [f64; 2],
    pub bottom_right: [f64; 2],
}

impl Rectangle {

    pub fn overlaps(&self, x: u32, y: u32, chunk_size: u32) -> bool {
        let chunk_bottom_right_x = x + chunk_size;
        let chunk_bottom_right_y = y + chunk_size;
        
        let condition1 = (self.bottom_right[0] as u32) < x;
        let condition2 = (self.top_left[0] as u32) > chunk_bottom_right_x;
        let condition3 = (self.bottom_right[1] as u32) < y;
        let condition4 = (self.top_left[1] as u32) > chunk_bottom_right_y;

        !(condition1 || condition2 || condition3 || condition4)
    }

    // Check if the point (x, y) lies inside this rectangle.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.top_left[0] as u32 &&
        x <= self.bottom_right[0] as u32 &&
        y >= self.top_left[1] as u32 &&
        y <= self.bottom_right[1] as u32
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub ignored_rectangles: Vec<Rectangle>,
}

impl Config {
    // This method returns a Vec of matching rectangles for a given page value

    // It *also* converts the x,y coordinates from inches to pixels
    // ...based on the height of the document in points (this app targets 2000 pixels for height of the image)
    
    pub fn get_matching_rectangles(&self, page: &str, height_in_points: i32) -> Vec<Rectangle> {
        let mut matching_rects = Vec::new();
    
        for rect in &self.ignored_rectangles {
            match rect.page.as_str() {
                "all" => matching_rects.push(rect.clone()),
                "even" => {
                    if let Ok(page_num) = page.parse::<i32>() {
                        if page_num % 2 == 0 {
                            matching_rects.push(rect.clone());
                        }
                    }
                },
                "odd" => {
                    if let Ok(page_num) = page.parse::<i32>() {
                        if page_num % 2 == 1 {
                            matching_rects.push(rect.clone());
                        }
                    }
                },
                page_val => {
                    if page == page_val {
                        matching_rects.push(rect.clone());
                    }
                }
            }
        }
    
        // Convert the x,y values from inches to pixels, based on the height of the PDF page

        // Determine pixels per point
        // 2000 / total height in points
        let pixels_per_point = 2000.0 / height_in_points as f64;
        // println!("Pixels per point calculated as: {}", pixels_per_point.to_string());

        // Convert the x,y values defining the rectangles from inches to pixels, using the points value
        // This conversion will vary based on the size of the PDF page.
        for rect in &mut matching_rects {

            // (x or y value in inches) * 72 points per inch * pixels_per_point
            // Top left x value
            rect.top_left[0] = ((rect.top_left[0] as f64) * (72 as f64) * (pixels_per_point)).round();
            // println!("Rect top left x: {}", rect.top_left[0].to_string());

            // Top left y value
            rect.top_left[1] = ((rect.top_left[1] as f64) * (72 as f64) * (pixels_per_point)).round();
            // println!("Rect top left y: {}", rect.top_left[1].to_string());

            // Bottom right x value
            rect.bottom_right[0] = ((rect.bottom_right[0] as f64) * (72 as f64) * (pixels_per_point)).round();
            // println!("Rect bottom right x: {}", rect.bottom_right[0].to_string());

            // Bottom right y value
            rect.bottom_right[1] = ((rect.bottom_right[1] as f64) * (72 as f64) * (pixels_per_point)).round();
            // println!("Rect bottom right y: {}", rect.bottom_right[1].to_string());

        }


        matching_rects
    }
}


// Structure for the result json output file
#[derive(Serialize, Deserialize)]
struct ComparisonResult {
    match_result: String,
}



fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Define global variables
    let mut differences_found_in_document: bool = false;
    let mut differences_found_in_page: bool;
    let mut differences_in_number_of_pages: bool = false;
    let mut config_json: Option<Config> = None;



    // Parse the command line arguments

    let cli = Cli::parse();

    
    // If the debug flag is set, print some flag and argument messages to the console
    if cli.debug {
        println!("The 'debug' flag was set.  More information will be provided at the console.");
    
        println!("pdf1: {}", cli.original_pdf1_path.display());
        println!("pdf2: {}", cli.original_pdf2_path.display());
    
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
    
        match cli.result {
            Some(ref value) => println!("The 'result' flag was set with value:  {}", value.to_string_lossy()),
            None => println!("The 'result' flag was not set."),
        }

        match cli.config {
            Some(ref value) => println!("The 'config' flag was set with value:  {}", value.to_string_lossy()),
            None => println!("The 'config' flag was not set."),
        }
    
    } 

    

    // ******************** License Logic ********************* //

    // Keygen Account ID
    let keygen_account_id = "ed7e781e-3c3f-4ecc-a451-3c40333c093e";


    // Create a fingerprint (UUID) for this run instance
    let instance_uuid = Uuid::new_v4();

    // Convert to a string
    let fingerprint_uuid = instance_uuid.to_string();

    if cli.debug {
        println!("Generated run instance UUID for license activation: {:?}", fingerprint_uuid);
    }

    // Local license configuration file work
    
    // Define some variables
    let license_config_folder = "C:\\ProgramData\\MatchPDF";
    let license_config_path = Path::new(license_config_folder).join("licenseConfig.dat");

    // If the config file exists
    if Path::new(&license_config_path).exists() {
        
        // Read the contents of the config file
        match fs::read_to_string(&license_config_path) {
            Ok(contents) => {
                match serde_json::from_str::<Value>(&contents) {
                    Ok(json) => {
                        
                        // Extract the license key
                        if let Some(license_key) = json["license_key"].as_str() {
                        
                            // Attempt to activate the license
                            match activate_license(keygen_account_id, &fingerprint_uuid, license_key) {
                                LicenseActivationResult::Success(msg) => println!("Success: {}", msg),
                                LicenseActivationResult::Error(e) => println!("Error: {}", e),
                                LicenseActivationResult::ValidationFailed(msg) => println!("Validation failed: {}", msg),
                                LicenseActivationResult::ActivationFailed(msg) => println!("Activation failed: {}", msg),
                            }
                        } else {
                            // Prompt for license key if JSON is invalid
                            println!("Stored license key information invalid.");
                            
                            get_and_store_license_key(license_config_path);
                            
                        }
                    },
                    Err(_) => {
                        // Prompt for license key if JSON parsing fails
                        println!("Stored license key information invalid.");

                        get_and_store_license_key(license_config_path);
                        
                    }
                }
            },
            Err(_) => {
                // Error handling for file read failure
                println!("File read failure for license config file.");
            }
        }
    } else {
        // File doesn't exist, prompt for license key
        println!("There is no local license key information.");

        get_and_store_license_key(license_config_path);
    }





    // // Create the folder if it doesn't exist
    // fs::create_dir_all(license_config_folder)?;

    // // Prepare the file, create it if it doesn't exist, or truncate it if it does
    // let mut license_config_file = OpenOptions::new()
    //     .write(true)
    //     .create(true)
    //     .truncate(true)
    //     .open(&license_config_path)?;

    // // JSON data to write
    // let license_data = json!({
    //     "license_key": "big-fat-key",
    //     "license_type": "paid-one-year"
    // });

    // // Write the JSON data to the file
    // license_config_file.write_all(license_data.to_string().as_bytes())?;





        // // activate_license(license_found_in_ProgramData, uuid)
        // let license_key = "2EE4AA-77081E-E79CE7-BA11F0-437F5F-V3";

        // match activate_license(keygen_account_id, &fingerprint_uuid, license_key) {
        //     LicenseActivationResult::Success(msg) => println!("Success: {}", msg),
        //     // LicenseActivationResult::Expired => println!("License expired"),
        //     LicenseActivationResult::Error(e) => println!("Error: {}", e),
        //     LicenseActivationResult::ValidationFailed(msg) => println!("Validation failed: {}", msg),
        //     LicenseActivationResult::ActivationFailed(msg) => println!("Activation failed: {}", msg),
        // }



        // if there was a failure to activate

            // Capture license from the user

    // Else
        
        // Capture license from the user




    // ******************** Command-Line Info Verification ********************* //

    // If the user provided an output file, check to see if the included folder exists
    if let Some(ref path) = cli.output {
        // Extract the parent directory of the provided path
        if let Some(parent_dir) = Path::new(path).parent() {
            // If the parent directory does not exist, exit now.
            if ! parent_dir.exists() {
                println!("The provided output folder does not exist.");
                process::exit(1);
            }
        } else {
            println!("Invalid output path provided.");
            process::exit(1);
        }
    }

    // If the user provided a result file, check to see if the included folder exists
    if let Some(ref path) = cli.result {
        // Extract the parent directory of the provided path
        if let Some(parent_dir) = Path::new(path).parent() {
            // If the parent directory does not exist, exit now.
            if ! parent_dir.exists() {
                println!("The provided result folder does not exist.");
                process::exit(1);
            }
        } else {
            println!("Invalid result path provided.");
            process::exit(1);
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
            config_json = Some(serde_json::from_str(&content).expect("\n\nFailed to deserialize JSON.\
                \n\nTips:\
                \nVerify that the config file contains a valid JSON object.\
                \nIf a value between 0 or 1 is desired for x or y, use a zero before the decimal.\n\n"));
            
            // println!("{:?}", config_json);

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
    .render_form_data(false);


    // Create a variable to hold the PDF document if it's needed
    let mut output_pdf = pdfium.create_new_pdf().unwrap();


    // If the number of pages in the two documents are the same, proceed with the comparison
    if ! differences_in_number_of_pages {

        // ... then iterate through the pages until reaching the end of the shortest document
        for index in 0..doc1_pages {

            // Reset variables
            differences_found_in_page = false;

            // Create a page differences vector variable at this scope level
            let page_differences_vector;

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

            let page_height_integer_in_points = doc1height.value.round() as i32;


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


            // Create a vector variable that will be passed into compare_images_in_chunks
            // This may be empty if there are no rectangles to ignore for this page
            let mut current_page_rectangles_to_ignore;
            
            // Define the current page number (index is base zero)
            let page_val = index + 1;

            // If there is a valid config json
            if let Some(temporary_config_json) = &config_json {

                // Check to see if there are rectangles that need to be ignored in this page
                current_page_rectangles_to_ignore = temporary_config_json.get_matching_rectangles(page_val.to_string().as_str(), page_height_integer_in_points);

                // println!("current_page_rectangles_to_ignore for page {:?}", page_val.to_string());
                // println!("{:?}", current_page_rectangles_to_ignore);

                // If there are rectangles that need to be ignored in this page
                if !current_page_rectangles_to_ignore.is_empty(){

                    // Compare the images of the two pages, sending in ignored areas
                    page_differences_vector = images::compare_images_in_chunks(&image1, &image2, Some(&current_page_rectangles_to_ignore));

                    // Set the differences_found variables to true if the vector is not empty
                    if !page_differences_vector.is_empty(){
                        differences_found_in_document = true;
                        differences_found_in_page = true;

                        if cli.debug {
                            println!("page_differences_vector for page {:?}: {:?}", page_val, page_differences_vector);
                        }

                    }


                } else { // There was a valid config JSON, but it did not contain ignored rectangles for this page

                    // Compare the images of the two pages, sending null for ignored rectangles
                    page_differences_vector = images::compare_images_in_chunks(&image1, &image2, None);

                    // Set the differences_found variables to true if the vector is not empty
                    if !page_differences_vector.is_empty(){
                        differences_found_in_document = true;
                        differences_found_in_page = true;

                        if cli.debug {
                            println!("page_differences_vector for page {:?}: {:?}", page_val, page_differences_vector);
                        }

                    }


                }


            } else {  // There was not valid config json, so don't worry about ignored rectangles - don't ignore anything.

                // Compare the images of the two pages, sending null for ignored rectangles
                page_differences_vector = images::compare_images_in_chunks(&image1, &image2, None);

                // Set the differences_found variables to true if the vector is not empty
                if !page_differences_vector.is_empty(){
                    differences_found_in_document = true;
                    differences_found_in_page = true;

                    if cli.debug {
                        println!("page_differences_vector for page {:?}: {:?}", page_val, page_differences_vector);
                    }

                }

                if cli.debug{    
                    println!("Config JSON is not initialized.");
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
                    let doc1_page_completed_image;
                    let doc2_page_completed_image;

                    // Highlight the differences within the images
                    // If differences were found in page
                    if differences_found_in_page {

                        doc1_page_highlighted_image = images::highlight_chunks(&image1, &page_differences_vector);

                        doc2_page_highlighted_image = images::highlight_chunks(&image2, &page_differences_vector);
                    
                        // Check for rectangles that were ignored - and draw them
                        if let Some(temporary_config_json) = &config_json {
                            
                            current_page_rectangles_to_ignore = temporary_config_json.get_matching_rectangles(page_val.to_string().as_str(), page_height_integer_in_points);

                            // If there are rectangles that were ignored on this page
                            if !current_page_rectangles_to_ignore.is_empty(){

                                // Draw them
                                doc1_page_completed_image = images::draw_ignored_rectangles(&doc1_page_highlighted_image, Some(&current_page_rectangles_to_ignore));
                                doc2_page_completed_image = images::draw_ignored_rectangles(&doc2_page_highlighted_image, Some(&current_page_rectangles_to_ignore));
                            
                            } else { // Else the highlighted image is the completed image

                                doc1_page_completed_image = doc1_page_highlighted_image;
                                doc2_page_completed_image = doc2_page_highlighted_image;

                            }

                        } else { // There was a valid config JSON, but it did not contain ignored rectangles for this page

                            // Make the highlighted images the completed images
                            doc1_page_completed_image = doc1_page_highlighted_image;
                            doc2_page_completed_image = doc2_page_highlighted_image;

                        }

                    } else { // Else there are not differences in this page, so the only thing to do is put the ignored rectangles on the page, if there are any.

                        //doc1_page_completed_image = image1;

                        //doc2_page_completed_image = image2;

                        // Check for rectangles that were ignored - and draw them
                        if let Some(temporary_config_json) = &config_json {
                            
                            current_page_rectangles_to_ignore = temporary_config_json.get_matching_rectangles(page_val.to_string().as_str(), page_height_integer_in_points);

                            // If there are rectangles that were ignored on this page
                            if !current_page_rectangles_to_ignore.is_empty(){

                                // Draw them onto the original, non-highlighted page images
                                doc1_page_completed_image = images::draw_ignored_rectangles(&image1, Some(&current_page_rectangles_to_ignore));
                                doc2_page_completed_image = images::draw_ignored_rectangles(&image2, Some(&current_page_rectangles_to_ignore));
                            
                            } else { // Else the original image is the completed image

                                doc1_page_completed_image = image1;
                                doc2_page_completed_image = image2;

                            }

                        } else { // There was a valid config JSON, but it did not contain ignored rectangles for this page

                            // Make the original, non-highlighted images the completed images
                            doc1_page_completed_image = image1;
                            doc2_page_completed_image = image2;

                        }
                        
                    }


                    // Create a single image that contains both highlighted images, as well as a separator
                    let total_width = doc1_page_completed_image.width() + doc2_page_completed_image.width() + 1;
                    let total_height = doc1_page_completed_image.height(); // assuming both images have the same height
                    let mut combined_image = RgbaImage::new(total_width, total_height);

                    // Copy the first image into the new image
                    image::imageops::replace(&mut combined_image, &doc1_page_completed_image, 0, 0);

                    // Draw the black line
                    for y in 0..total_height {
                        combined_image.put_pixel(doc1_page_completed_image.width(), y, Rgba([0, 0, 0, 255]));
                    }

                    // Copy the second image next to the black line
                    image::imageops::replace(&mut combined_image, &doc2_page_completed_image, doc1_page_completed_image.width() as i64 + 1, 0);


                    ///////
                    // Begin creating the page that will be added to the output PDF
                    ///////

                    // PDF documents use points as a unit of measurement, and there are 72 points to an inch.
                    const POINTS_PER_INCH: f32 = 72.0;

                    // Desired page width in inches
                    let desired_width_in_inches = 17.0;  // For example: Letter width

                    // Calculate the desired width in points
                    let desired_width_in_points = desired_width_in_inches * POINTS_PER_INCH;

                    // Calculate the scaling factor based on the desired width
                    let scale_factor = desired_width_in_points / combined_image.width() as f32;

                    // Apply the scaling factor to image sizes and positions
                    let width = combined_image.width() as f32 * scale_factor;
                    let height = combined_image.height() as f32 * scale_factor;
                    
                    let paper_size = PdfPagePaperSize::Custom(PdfPoints::new(width), PdfPoints::new(height));

                    let image_x_position_in_points = PdfPoints::new(0.0);
                    let image_y_position_in_points = PdfPoints::new(0.0);


                    // Add a page to the output pdf document
                    let page = output_pdf.pages_mut().create_page_at_end(paper_size);

                    // Check to see if the page is a page, since it was actually wrapped in a result enum
                    if let Ok(mut page) = page {

                        // Get the combined image width
                        let combined_image_width = combined_image.width() as f32 * scale_factor;

                        // Convert the combined image into the type acceptable for writing to the page
                        let dynamic_combined_image = DynamicImage::ImageRgba8(combined_image);

                        // Make a PDF document object using the combined image
                        let mut object = PdfPageImageObject::new_with_width(
                            &output_pdf,
                            &dynamic_combined_image,
                            PdfPoints::new(combined_image_width),
                        )?;

                        // Describe the placement of the object (start from 0,0 as it's a single image)
                        object.translate(image_x_position_in_points, image_y_position_in_points)?;

                        // Add the combined image to the destination PDF page.
                        page.objects_mut().add_image_object(object)?;

                    } else {

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

    // If the user used the 'output' argument
    if let Some(ref _value) = cli.output {

        // ...and there is at least one page in the output file
        // (consider combinations like 'justdiff' and files matching)
        if output_pdf.pages().get(0).is_ok() {

            // Write the document to disk
            if let Some(ref path) = cli.output {
                output_pdf.save_to_file(path)?;
            } else {
                println!("There is an issue with the file path provided as the output.");
            }
        
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


    // If a result text (json) file is desired, write to it.
    if let Some(ref _value) = cli.result {

        // Set the text you want to write to the JSON
        let result_text: String;

        if differences_found_in_document || differences_in_number_of_pages {

            result_text = "Differences were found".to_string();

        } else {

            result_text = "Documents match".to_string();

        }

        // Create a variable to hold the text result of the comparison
        let result = ComparisonResult {
            match_result: result_text,
        };

        // Serialize the result
        let file_content = serde_json::to_string_pretty(&result).unwrap();

        // Write serialized content to file using the user-specified PathBuf
        let mut file = match File::create(_value) {  // Here, use _value which is a reference to the inner PathBuf
            Ok(file) => file,
            Err(e) => {
                println!("Error creating file: {}", e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Error creating file",
                )));
            }
        };

        // Write the content to the file, using the file object:
        if let Err(e) = file.write_all(file_content.as_bytes()) {
            println!("Error writing to file: {}", e);
        }
    }


    Ok(())

}




// Custom Enum to define results coming out of the activate_license function
enum LicenseActivationResult {
    Success(String),
    Error(Box<dyn Error>),
    ValidationFailed(String),
    ActivationFailed(String),
}



// Activate License Function
// Function to activate a software license using the Keygen API
// Emulating https://github.com/keygen-sh/example-python-machine-activation/blob/master/main.py
// Send MatchPDF keygenAccountID, UUID fingerprint, and this customer's license key

fn activate_license(keygen_account_id: &str, fingerprint: &str, license_key: &str) -> LicenseActivationResult {

    // Create a new HTTP client instance
    let client = Client::new();

    // Attempt to validate the license key with the Keygen API
    let validation_resp = match client.post(&format!(
        "https://api.keygen.sh/v1/accounts/{}/licenses/actions/validate-key",
        keygen_account_id
    ))
    .json(&json!({
        "meta": {
            "scope": { "fingerprint": fingerprint },
            "key": license_key
        }
    }))
    .send() {
        Ok(resp) => resp,
        Err(e) => return LicenseActivationResult::Error(Box::new(e)),
    };

    // Parse the JSON response for validation
    let validation = match validation_resp.json::<Value>() {
        Ok(v) => v,
        Err(e) => return LicenseActivationResult::Error(Box::new(e)),
    };

    // Check for errors in the validation response and handle them.
    if validation.get("errors").is_some() {
        let errs = validation["errors"].as_array().unwrap();
        let error_messages: Vec<String> = errs
            .iter()
            .map(|e| format!("{} - {}", e["title"], e["detail"]).to_lowercase())
            .collect();

        return LicenseActivationResult::ValidationFailed(format!("License validation failed: {:?}", error_messages));
    }

    // Check if the license is already active
    if validation["meta"]["valid"].as_bool().unwrap_or(false) {
        return LicenseActivationResult::Success("License has already been activated on this machine".to_string());
    }

    // Determine if activation is required based on the activation code
    let validation_code = validation["meta"]["code"].as_str().unwrap_or("");
    let activation_is_required = match validation_code {
        "FINGERPRINT_SCOPE_MISMATCH" | "NO_MACHINES" | "NO_MACHINE" => true,
        _ => false,
    };

    // Handle cases where activation is not required
    if !activation_is_required {
        return LicenseActivationResult::ValidationFailed(format!("License {}", validation["meta"]["detail"].as_str().unwrap_or_default()));
    }

    // Activate the machine with the license if required
    let activation_resp = match client.post(&format!(
        "https://api.keygen.sh/v1/accounts/{}/machines",
        keygen_account_id
    ))
    .json(&json!({
        "data": {
            "type": "machines",
            "attributes": {
                "fingerprint": fingerprint
            },
            "relationships": {
                "license": {
                    "data": { "type": "licenses", "id": validation["data"]["id"] }
                }
            }
        }
    }))
    .header("Authorization", format!("License {}", license_key))
    .send() {
        Ok(resp) => resp,
        Err(e) => return LicenseActivationResult::Error(Box::new(e)),
    };

    // Parse the JSON response for activation
    let activation = match activation_resp.json::<Value>() {
        Ok(a) => a,
        Err(e) => return LicenseActivationResult::Error(Box::new(e)),
    };

    // Check for errors in the activation response and handle them
    if activation.get("errors").is_some() {
        let errs = activation["errors"].as_array().unwrap();
        let error_messages: Vec<String> = errs
            .iter()
            .map(|e| format!("{} - {}", e["title"], e["detail"]).to_lowercase())
            .collect();

        return LicenseActivationResult::ActivationFailed(format!("License activation failed: {:?}", error_messages));
    }

    // Return success if the license is successfully activated
    LicenseActivationResult::Success("License activated for this machine".to_string())
}


// Function to prompt for and retrieve the license key from the user
fn get_and_store_license_key(license_config_path: PathBuf) {
    let mut license_key = String::new();

    // Keep prompting until a valid license key is entered
    loop {
        print!("Please enter your license key: ");
        io::stdout().flush().unwrap(); // Ensure the prompt is displayed immediately
        io::stdin().read_line(&mut license_key).expect("Failed to read line");

        // Trim newline and whitespace
        license_key = license_key.trim().to_string();

        // Check if the license key is non-zero length
        if !license_key.is_empty() {
            break; // Break the loop if the license key is valid
        } else {
            println!("Invalid input. License key cannot be empty.\n");
        }
    }

    // Create the JSON object
    let license_data = json!({
        "license_key": &license_key,
        // Add other fields if necessary
    });

    // Write to the file
    {
        // Ensure the directory exists
        if let Some(parent) = license_config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).expect("Failed to create config directory.  Be sure this app and user have write access to C:\\ProgramData");
            }
        }
    
        // Attempt to open or create the file
        match File::create(&license_config_path) {
            Ok(_) => {
                // Write the license key to the file
                fs::write(&license_config_path, serde_json::to_string(&license_data).unwrap())
                    .expect("Failed to write to the config file");
            },
            Err(e) => {
                // Handle specific error if the file does not exist
                if e.kind() != ErrorKind::AlreadyExists {
                    panic!("Failed to create the config file: {}", e);
                }
            }
        }
    }


}