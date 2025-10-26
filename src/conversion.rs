use image::imageops::{self, contrast};
use image::{DynamicImage, ImageBuffer, Luma, Rgb};
use imageproc::contours::find_contours;
use imageproc::contrast::equalize_histogram_mut;
use imageproc::drawing::draw_filled_circle_mut;
use imageproc::edges::canny;
use imageproc::filter::{bilateral_filter, median_filter};
use imageproc::geometry::min_area_rect;
use imageproc::map::map_colors;
use imageproc::point::Point;

use crate::io;

const BLACK_BORDER_THRESHOLD: u8 = 30;
const WHITE_LIGHT_THRESHOLD: u8 = 245;

pub fn convert(
    image: &ImageBuffer<Rgb<u16>, Vec<u16>>,
    debug_file_path: Option<&str>,
) -> ImageBuffer<Rgb<u16>, Vec<u16>> {
    let mut img: DynamicImage = image.clone().into();

    if image.width() > 500 || image.height() > 500 {
        // use a smaller image for faster processing
        img = img.resize(500, 500, imageops::FilterType::Triangle);
    }

    // convert to grayscale
    let mut img = img.to_luma8();

    if let Some(path) = debug_file_path {
        io::save_image(path, "grayscale", "jpeg", img.clone());
    }

    // remove any black borders (e.g. from the edges of a film holder)
    // these should remain fairly dark after denoising
    equalize_histogram_mut(&mut img);
    img = map_colors(&img, |p| {
        if p.0[0] < BLACK_BORDER_THRESHOLD || p.0[0] > WHITE_LIGHT_THRESHOLD {
            Luma([255])
        } else {
            p
        }
    });
    // eliminate any remaining ~1px-sized border specks
    img = median_filter(&img, 5, 5);

    // denoise
    // img = contrast(&img, 30.0);
    equalize_histogram_mut(&mut img);
    img = bilateral_filter(&img, 10, 3.0, 5.0);

    if let Some(path) = debug_file_path {
        io::save_image(path, "borderless", "jpeg", img.clone());
    }

    // find edges
    img = contrast(&img, 50.0);
    img = canny(&img, 3.0, 100.0);

    if let Some(path) = debug_file_path {
        io::save_image(path, "edges", "jpeg", img.clone());
    }

    // find contours
    let contours = find_contours::<u32>(&img);
    let points: Vec<Point<u32>> = contours
        .into_iter()
        .flat_map(|contour| contour.points)
        .collect();

    let corners = min_area_rect(&points);

    let resized_width = img.width() as f32;
    let resized_height = img.height() as f32;

    let mut img = image.clone();
    for corner in corners {
        let x = (image.width() as f32 * corner.x as f32 / resized_width) as i32;
        let y = (image.height() as f32 * corner.y as f32 / resized_height) as i32;

        draw_filled_circle_mut(&mut img, (x, y), 50, Rgb([255, 0, 0]));
    }

    if let Some(path) = debug_file_path {
        io::save_image(path, "corners", "jpeg", img.clone());
    }

    img
}
