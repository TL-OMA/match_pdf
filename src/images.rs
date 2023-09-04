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