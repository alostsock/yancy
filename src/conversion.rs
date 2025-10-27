use image::imageops::{self, contrast};
use image::{DynamicImage, ImageBuffer, Luma, Rgb};
use imageproc::contours::find_contours;
use imageproc::drawing::draw_line_segment_mut;
use imageproc::edges::canny;
use imageproc::filter::median_filter;
use imageproc::geometry::min_area_rect;
use imageproc::map::map_colors;
use imageproc::point::Point;

use crate::io;
use crate::processing::normalize_histogram_mut;

const BLACK_BORDER_THRESHOLD: u8 = 20;
const WHITE_LIGHT_THRESHOLD: u8 = 240;

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

    // 1. normalize histogram for more consistent black/white values
    normalize_histogram_mut(&mut img);

    if let Some(path) = debug_file_path {
        io::save_image(path, "grayscale", "jpeg", img.clone());
    }

    // 2. zero out any black borders or light from sprocket holes
    img = map_colors(&img, |p| {
        if p.0[0] < BLACK_BORDER_THRESHOLD || p.0[0] > WHITE_LIGHT_THRESHOLD {
            Luma([0])
        } else {
            p
        }
    });

    // 3. the brightest values should now mainly be from the film backing.
    // re-normalize these values, since they should be brighter now
    normalize_histogram_mut(&mut img);

    // 4. change the values from step (2) to white, in preparation for edge
    // detection
    img = map_colors(&img, |p| {
        if p.0[0] < BLACK_BORDER_THRESHOLD {
            Luma([255])
        } else {
            p
        }
    });

    // 5. remove any specks of black remaining from step (2)
    img = median_filter(&img, 1, 1);

    if let Some(path) = debug_file_path {
        io::save_image(path, "borderless", "jpeg", img.clone());
    }

    // 6. find edges
    img = contrast(&img, 50.0);
    img = canny(&img, 3.0, 100.0);

    if let Some(path) = debug_file_path {
        io::save_image(path, "edges", "jpeg", img.clone());
    }

    // 7. find contours
    let contours = find_contours::<u32>(&img);
    let points: Vec<Point<u32>> = contours
        .into_iter()
        .filter_map(|contour| {
            if contour.points.len() > 40 {
                Some(contour.points)
            } else {
                None
            }
        })
        .flatten()
        .collect();

    let corners = min_area_rect(&points);

    let resized_width = img.width() as f32;
    let resized_height = img.height() as f32;

    if let Some(path) = debug_file_path {
        let mut points = corners.clone().to_vec();
        points.push(points[0]);

        let mut img = image.clone();
        for i in 1..points.len() {
            let x0 = image.width() as f32 * points[i - 1].x as f32 / resized_width;
            let y0 = image.height() as f32 * points[i - 1].y as f32 / resized_height;
            let x1 = image.width() as f32 * points[i].x as f32 / resized_width;
            let y1 = image.height() as f32 * points[i].y as f32 / resized_height;

            draw_line_segment_mut(&mut img, (x0, y0), (x1, y1), Rgb([0, u16::MAX, u16::MAX]));
        }

        io::save_image(path, "bounds", "jpeg", img);
    }

    image.clone()
}
