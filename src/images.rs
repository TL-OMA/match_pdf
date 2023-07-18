// Image related functions

use pdfium_render::prelude::*;
use image::RgbaImage;
use image::ImageBuffer;


/// Given a pdf document object (loaded pdf) and page number, return an image of the page
pub fn render_page(page: &PdfPage, render_config: &PdfRenderConfig) -> Result<RgbaImage, PdfiumError> {
    let binding = page.render_with_config(render_config)?
        .as_image();
    let image = binding // Renders this page to an image::DynamicImage...
        .as_rgba8() // ... then converts it to an image::Image...
        .ok_or(PdfiumError::ImageError)?;

    Ok(image.clone())
}


/// Compare two images, tracking chunks to make the later highlighting easier
pub fn compare_images_in_chunks(img1: &ImageBuffer<image::Rgba<u8>, Vec<u8>>, img2: &ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> Vec<(u32, u32)> {
    let chunk_size = 10;
    let mut differing_chunks = vec![];

    // Assumes both images have same dimensions
    let (width, height) = img1.dimensions();

    for y in (0..height).step_by(chunk_size) {
        for x in (0..width).step_by(chunk_size) {
            let mut chunks_differ = false;

            for dy in 0..chunk_size {
                for dx in 0..chunk_size {
                    // Don't try to access a pixel that is out of the image's bounds
                    if x + (dx as u32) >= width || y + (dy as u32) >= height {
                    //if x + dx >= width || y + dy >= height {
                        continue;
                    }

                    let img1_pixel = img1.get_pixel(x + (dx as u32), y + (dy as u32));
                    let img2_pixel = img2.get_pixel(x + (dx as u32), y + (dy as u32));

                    if img1_pixel != img2_pixel {
                        chunks_differ = true;
                        break;
                    }
                }
                if chunks_differ {
                    break;
                }
            }

            if chunks_differ {
                differing_chunks.push((x, y));
            }
        }
    }

    differing_chunks
}
