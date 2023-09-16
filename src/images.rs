// Image related functions

use pdfium_render::prelude::*;
use image::RgbaImage;
use image::{ImageBuffer, Rgba};
// Use the Rectangle struct in main.rs
use crate::Rectangle;


/// Given a pdf document object (loaded pdf) and page number, return an image of the page
pub fn render_page(page: &PdfPage, render_config: &PdfRenderConfig) -> Result<RgbaImage, PdfiumError> {
    let binding = page.render_with_config(render_config)?
        .as_image();
    let image = binding // Renders this page to an image::DynamicImage...
        .as_rgba8() // ... then converts it to an image::Image...
        .ok_or(PdfiumError::ImageError)?;

    Ok(image.clone())
}


pub fn compare_images_in_chunks(
    img1: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    img2: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    ignore_rects: Option<&Vec<Rectangle>>,
) -> Vec<(u32, u32)> {
    let chunk_size = 10;
    let mut differing_chunks = vec![];

    let (width, height) = img1.dimensions();

    // Iterate through each chunk in the images.
    for y in (0..height).step_by(chunk_size) {
        for x in (0..width).step_by(chunk_size) {
            
            // Flags to keep track of chunk status relative to ignore rectangles.
            let mut is_chunk_ignored = false; // Completely inside ignore rectangle?
            let mut is_chunk_partial = false; // Partially overlaps with ignore rectangle?

            // Check if the current chunk overlaps or is inside any of the ignore rectangles.
            if let Some(rects) = ignore_rects {
                for rect in rects.iter() {
                    if rect.overlaps(x, y, chunk_size as u32) {
                        if rect.contains(x, y) && rect.contains(x + chunk_size as u32, y + chunk_size as u32) {
                            is_chunk_ignored = true;
                            break;
                        } else {
                            is_chunk_partial = true;
                        }
                    }
                }
            }

            // If the chunk is fully inside an ignore rectangle, move to the next chunk.
            if is_chunk_ignored {
                continue;
            }

            let mut chunks_differ = false;

            // Compare each pixel inside the chunk.
            for dy in 0..chunk_size {
                for dx in 0..chunk_size {
                    let actual_x = x + (dx as u32);
                    let actual_y = y + (dy as u32);

                    // Ensure the pixel coordinates are within image dimensions.
                    if actual_x >= width || actual_y >= height {
                        continue;
                    }

                    // For chunks that partially overlap with ignore rectangles, 
                    // skip pixels that are inside those rectangles.
                    if is_chunk_partial {
                        if let Some(rects) = ignore_rects {
                            if rects.iter().any(|rect| rect.contains(actual_x, actual_y)) {
                                continue;
                            }
                        }
                    }

                    // Get pixels from both images.
                    let img1_pixel = img1.get_pixel(actual_x, actual_y);
                    let img2_pixel = img2.get_pixel(actual_x, actual_y);

                    // If a differing pixel is found, mark the chunk as different and break.
                    if img1_pixel != img2_pixel {
                        chunks_differ = true;
                        break;
                    }
                }

                if chunks_differ {
                    break;
                }
            }

            // If the chunk contains differing pixels, add it to the result list.
            if chunks_differ {
                differing_chunks.push((x, y));
            }
        }
    }

    differing_chunks
}



// Highlight the differing chunks within the image
pub fn highlight_chunks(image: &ImageBuffer<Rgba<u8>, Vec<u8>>, chunks: &[(u32, u32)]) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let mut new_image = image.clone();

    for &(x, y) in chunks {
        for dx in 0..10 {
            for dy in 0..10 {
                let new_x = x + dx;
                let new_y = y + dy;

                // Check if the pixel is in the image
                if new_x < width && new_y < height {
                    let pixel = new_image.get_pixel_mut(new_x, new_y);

                    // If the pixel is dark
                    if pixel[0] < 150 && pixel[1] < 150 && pixel[2] < 150 {
                        // Change it appropriately (Light Pink)
                        pixel[0] = 249;
                        pixel[1] = 133;
                        pixel[2] = 139;

                    // ...else if the pixel is light 
                    } else if pixel[0] > 215 && pixel[1] < 215 && pixel[2] < 215{
                        // Change it appropriately (Hot Pink)
                        pixel[0] = 118;
                        pixel[1] = 17;
                        pixel[2] = 55; 


                    } else {
                        // Make all other pixels Maroon
                        pixel[0] = 237;
                        pixel[1] = 51;
                        pixel[2] = 95; 

                    }
                }
            }
        }
    }

    new_image
}


// Draw the ignored rectangles on the image
pub fn draw_ignored_rectangles(image: &ImageBuffer<Rgba<u8>, Vec<u8>>, ignore_rects: Option<&Vec<Rectangle>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut new_image = image.clone();
    
    // Check if rectangles are provided
    if let Some(rectangles) = ignore_rects {
        // Iterate over each rectangle
        for rect in rectangles {
            
            let top_left = rect.top_left;
            let bottom_right = rect.bottom_right;

            // Draw the top and bottom borders of the rectangle:
            // Loop from the leftmost to the rightmost x-coordinate of the rectangle.
            // The x,y values are currently f64, so round them and make them ints
            for x in (top_left[0].round() as i32)..(bottom_right[0].round() as i32) {
                // Make sure we're not going out of the image's width boundaries
                if x >= 0 && x < new_image.width() as i32 {
                    // Set the top border's pixel color
                    set_ignored_pixel_border_color(&mut new_image, x, top_left[1].round() as i32);
                    // Set the bottom border's pixel color
                    set_ignored_pixel_border_color(&mut new_image, x, bottom_right[1].round() as i32);
                }
            }

            // Draw the left and right borders of the rectangle:
            // Loop from the top to the bottom y-coordinate of the rectangle, excluding the corners.
            for y in (top_left[1].round() as i32 + 1)..bottom_right[1].round() as i32 {
                // Make sure we're not going out of the image's height boundaries
                if y >= 0 && y < new_image.height() as i32 {
                    // Set the left border's pixel color
                    set_ignored_pixel_border_color(&mut new_image, top_left[0].round() as i32, y);
                    // Set the right border's pixel color
                    set_ignored_pixel_border_color(&mut new_image, bottom_right[0].round() as i32, y);
                }
            }
        }
    }

    // Return the modified image
    new_image
}


// Helper function to set a specific pixel's color
fn set_ignored_pixel_border_color(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, y: i32) {
    // Check if the given coordinates are within the image boundaries
    if x >= 0 && x < image.width() as i32 && y >= 0 && y < image.height() as i32 {

        // Set every other pixel one color, then the others a different color
        if (x + y) % 2 == 0 {
            let pixel = image.get_pixel_mut(x as u32, y as u32);
            pixel[0] = 255;  // Set the Red channel
            pixel[1] = 0;    // Set the Green channel
            pixel[2] = 0;    // Set the Blue channel
        
        } else {
            let pixel = image.get_pixel_mut(x as u32, y as u32);
            pixel[0] = 0;  // Set the Red channel
            pixel[1] = 0;    // Set the Green channel
            pixel[2] = 0;    // Set the Blue channel
        }
        
    }
}