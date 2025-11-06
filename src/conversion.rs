use std::collections::HashMap;
use std::u16;

use image::imageops::{self, contrast, crop_imm};
use image::{DynamicImage, GrayImage, ImageBuffer, ImageError, Luma, Rgb};
use imageproc::contours::find_contours;
use imageproc::drawing::{draw_filled_circle_mut, draw_line_segment_mut};
use imageproc::edges::canny;
use imageproc::filter::median_filter;
use imageproc::geometry::min_area_rect;
use imageproc::map::{map_colors, map_colors_mut};
use imageproc::point::Point;

use crate::histogram::{normalize_histogram_mut, stretch_channels_mut};
use crate::io;

const BLACK_BORDER_THRESHOLD: u8 = 20;
const WHITE_LIGHT_THRESHOLD: u8 = 240;

pub type InputImage = ImageBuffer<Rgb<u16>, Vec<u16>>;

type Bounds = (u32, u32, u32, u32);

struct Border {
    bounds: Bounds,
    points: Vec<(u32, u32)>,
}

pub fn convert(
    original: &InputImage,
    aspect_ratio: f32,
    crop_percentage: f32,
    clip_percentage: f32,
    debug_file_path: Option<&str>,
) -> Result<InputImage, ImageError> {
    let Border {
        bounds,
        points: border_points,
    } = identify_border(&original, debug_file_path)?;

    let (min_x, min_y, max_x, max_y) =
        determine_crop_inset_bounds(original, bounds, aspect_ratio, crop_percentage);

    if let Some(path) = debug_file_path {
        let mut img = original.clone();

        let points = vec![
            (min_x as f32, min_y as f32),
            (max_x as f32, min_y as f32),
            (max_x as f32, max_y as f32),
            (min_x as f32, max_y as f32),
            (min_x as f32, min_y as f32),
        ];
        for i in 1..points.len() {
            let p0 = points[i - 1];
            let p1 = points[i];
            draw_line_segment_mut(&mut img, p0, p1, Rgb([0, u16::MAX, u16::MAX]));
        }

        for &(x, y) in border_points.iter() {
            draw_filled_circle_mut(&mut img, (x as i32, y as i32), 10, Rgb([u16::MAX, 0, 0]));
        }

        io::save_image(path, "border", "jpeg", img)?;
    }

    let border_colors: Vec<&Rgb<u16>> = border_points
        .iter()
        .map(|&(x, y)| original.get_pixel(x, y))
        .collect();

    let avg_border_color = Rgb([
        rms(border_colors.iter().map(|c| c.0[0]).collect()),
        rms(border_colors.iter().map(|c| c.0[1]).collect()),
        rms(border_colors.iter().map(|c| c.0[2]).collect()),
    ]);

    let mut output = crop_border(original, min_x, min_y, max_x, max_y);

    white_balance(&mut output, avg_border_color);

    invert_mut(&mut output);

    if let Some(path) = debug_file_path {
        io::save_image(path, "inverted", "jpeg", output.clone())?;
    }

    stretch_channels_mut(&mut output, clip_percentage);

    Ok(output)
}

fn identify_border(
    original: &InputImage,
    debug_file_path: Option<&str>,
) -> Result<Border, ImageError> {
    let mut img: DynamicImage = original.clone().into();

    if original.width() > 500 || original.height() > 500 {
        // use a smaller image for faster processing
        img = img.resize(500, 500, imageops::FilterType::Triangle);
    }

    // convert to grayscale
    let mut img: GrayImage = img.to_luma8();

    // 1. normalize histogram for more consistent black/white values
    normalize_histogram_mut(&mut img);

    if let Some(path) = debug_file_path {
        io::save_image(path, "grayscale", "jpeg", img.clone())?;
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

    if let Some(path) = debug_file_path {
        io::save_image(path, "pre-border-removal", "jpeg", img.clone())?;
    }

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
    let borderless = median_filter(&img, 1, 1);

    if let Some(path) = debug_file_path {
        io::save_image(path, "borderless", "jpeg", borderless.clone())?;
    }

    // 6. find edges
    img = contrast(&borderless, 50.0);
    img = canny(&img, 1.0, 50.0);

    if let Some(path) = debug_file_path {
        io::save_image(path, "edges", "jpeg", img.clone())?;
    }

    // 7. find contours
    let contours = find_contours::<u32>(&img);
    // use signed ints, since `min_area_react` may return negative coordinates
    let points: Vec<Point<i32>> = contours
        .into_iter()
        .filter_map(|contour| {
            if contour.points.len() > 50 {
                Some(contour.points)
            } else {
                None
            }
        })
        .flatten()
        .map(|p| Point {
            x: p.x as i32,
            y: p.y as i32,
        })
        .collect();

    let corners: [Point<i32>; 4] = min_area_rect(&points);

    let min_x = corners.map(|c| c.x).into_iter().min().unwrap().max(0) as u32;
    let min_y = corners.map(|c| c.y).into_iter().min().unwrap().max(0) as u32;
    let max_x = corners.map(|c| c.x).into_iter().max().unwrap().max(0) as u32;
    let max_y = corners.map(|c| c.y).into_iter().max().unwrap().max(0) as u32;

    let points = identify_border_points(
        min_x,
        min_y,
        max_x,
        max_y,
        &borderless
    );

    let scale_x = |x: u32| (x as f32 * original.width() as f32 / img.width() as f32) as u32;
    let scale_y = |y: u32| (y as f32 * original.height() as f32 / img.height() as f32) as u32;

    Ok(Border {
        bounds: (
            scale_x(min_x),
            scale_y(min_y),
            scale_x(max_x),
            scale_y(max_y),
        ),
        points: points
            .into_iter()
            .map(|(x, y)| (scale_x(x), scale_y(y)))
            .collect(),
    })
}

fn identify_border_points(
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    img: &GrayImage,
) -> Vec<(u32, u32)> {
    let mut pixel_positions: HashMap<u8, Vec<(u32, u32)>> = HashMap::new();
    let mut hist = vec![0u8; 256];

    let gap_x = (img.width() as f32 * 0.05) as u32;
    let gap_y = (img.height() as f32 * 0.05) as u32;

    let mut i = 0;
    for &p in img.iter() {
        let x = i % img.width();
        let y = i / img.width();
        if (x < min_x + gap_x || x > max_x - gap_x)
            && (y < min_y + gap_y || y > max_y - gap_y)
            && p != 255
        {
            hist[p as usize] += 1;
            pixel_positions
                .entry(p)
                .and_modify(|positions| positions.push((x, y)))
                .or_insert(vec![(x, y)]);
        }
        i += 1;
    }

    let border_value = hist
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(i, _)| i)
        .unwrap() as u8;

    pixel_positions.remove(&border_value).unwrap()
}

/// root mean square
fn rms(values: Vec<u16>) -> u16 {
    let sum: usize = values.iter().map(|&v| (v as usize).pow(2)).sum();
    (sum as f32 / values.len() as f32).sqrt() as u16
}

fn crop_border(img: &InputImage, min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> InputImage {
    crop_imm(img, min_x, min_y, max_x - min_x, max_y - min_y).to_image()
}

/// Combines these approaches:
/// https://stackoverflow.com/questions/54470148/white-balance-a-photo-from-a-known-point
/// https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color
fn white_balance(img: &mut InputImage, white_color: Rgb<u16>) {
    let lum = rms(vec![
        (0.299_f32.sqrt() * white_color.0[0] as f32) as u16,
        (0.587_f32.sqrt() * white_color.0[1] as f32) as u16,
        (0.114_f32.sqrt() * white_color.0[2] as f32) as u16,
    ]) as f32;

    let ratio_r = lum / white_color.0[0] as f32;
    let ratio_g = lum / white_color.0[1] as f32;
    let ratio_b = lum / white_color.0[2] as f32;

    map_colors_mut(img, |p| {
        Rgb([
            (p.0[0] as f32 * ratio_r) as u16,
            (p.0[1] as f32 * ratio_g) as u16,
            (p.0[2] as f32 * ratio_b) as u16,
        ])
    })
}

fn invert_mut(img: &mut InputImage) {
    map_colors_mut(img, |p| {
        Rgb([
            (u16::MAX - p.0[0]),
            (u16::MAX - p.0[1]),
            (u16::MAX - p.0[2]),
        ])
    });
}

pub fn split_image(img: InputImage) -> [InputImage; 2] {
    [
        crop_imm(&img, 0, 0, img.width() / 2, img.height()).to_image(),
        crop_imm(&img, img.width() / 2, 0, img.width() / 2, img.height()).to_image(),
    ]
}

pub fn determine_crop_inset_bounds(
    original: &InputImage,
    (min_x, min_y, max_x, max_y): Bounds,
    aspect_ratio: f32,
    crop_percentage: f32,
) -> Bounds {
    let current_aspect_ratio = (max_x - min_x) as f32 / (max_y - min_y) as f32;

    let inset_x = (original.width() as f32 * crop_percentage / 2.0) as u32;
    let inset_y = (original.height() as f32 * crop_percentage / 2.0) as u32;

    if current_aspect_ratio > aspect_ratio {
        // current bounds are too wide. height is OK
        let target_width = (max_y - min_y - inset_y * 2) as f32 * aspect_ratio;
        let mid_x = (max_x - min_x) as f32 / 2.0;
        let left = min_x + (mid_x - target_width / 2.0) as u32;
        let right = min_x + (mid_x + target_width / 2.0) as u32;
        (left, min_y + inset_y, right, max_y - inset_y)
    } else if current_aspect_ratio < aspect_ratio {
        // current bounds are too tall. width is OK
        let target_height = (max_x - min_x - inset_x * 2) as f32 / aspect_ratio;
        let mid_y = (max_y - min_y) as f32 / 2.0;
        let top = min_y + (mid_y - target_height / 2.0) as u32;
        let bottom = min_y + (mid_y + target_height / 2.0) as u32;
        (min_x + inset_x, top, max_x - inset_x, bottom)
    } else {
        (
            min_x + inset_x,
            min_y + inset_y,
            max_x - inset_x,
            max_y - inset_y,
        )
    }
}
