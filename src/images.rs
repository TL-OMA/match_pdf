// Image related functions

use pdfium_render::prelude::*;
use image::RgbaImage;


/// Given a pdf document object (loaded pdf) and page number, return an image of the page
pub fn render_page(page: &PdfPage, render_config: &PdfRenderConfig) -> Result<RgbaImage, PdfiumError> {
    let binding = page.render_with_config(render_config)?
        .as_image();
    let image = binding // Renders this page to an image::DynamicImage...
        .as_rgba8() // ... then converts it to an image::Image...
        .ok_or(PdfiumError::ImageError)?;

    Ok(image.clone())
}