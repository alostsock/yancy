use std::ops::Deref;

use image::{DynamicImage, EncodableLayout, ImageBuffer, Pixel, PixelWithColorType};

pub fn save_image<'a, P, Container>(path: &str, suffix: &str, extension: &str, image: ImageBuffer<P, Container>)
where
    P: Pixel + PixelWithColorType,
    [P::Subpixel]: EncodableLayout,
    Container: Deref<Target = [P::Subpixel]>,
    ImageBuffer<P, Container>: Into<DynamicImage>
{
    let output_path = format!("{}.{}.{}", path, suffix, extension);

    if let Err(_) = image.save(&output_path) {
        if let Err(e) = Into::<DynamicImage>::into(image).to_rgb8().save(&output_path) {
            eprintln!("Failed to save image: {}", e);
            std::process::exit(1);
        } else {
            println!("Saved (8-bit rgb) {}", output_path);
        }
    } else {
        println!("Saved {}", output_path);
    }
}
