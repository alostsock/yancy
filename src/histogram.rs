use std::io::Write;

use image::GrayImage;
use imageproc::stats::cumulative_histogram;
use nalgebra::DVector;
use rayon::prelude::*;

use crate::conversion::InputImage;
use crate::polyfit;

/// imageproc's `equalize_histogram_mut` doesn't preserve black or white levels.
/// Here, we keep track of the `min` CDF value so that pixels with values 0 and
/// 255 remain 0 and 255, respectively.
pub fn normalize_histogram_mut(image: &mut GrayImage) {
    let hist = cumulative_histogram(image).channels[0];
    let min = hist[0] as f32;
    let max = u8::MAX as f32;
    let total = hist[255] as f32;

    image.par_iter_mut().for_each(|p| {
        // JUSTIFICATION
        //  Benefit
        //      Using checked indexing here makes this function take 1.1x longer, as measured
        //      by bench_equalize_histogram_mut
        //  Correctness
        //      Each channel of CumulativeChannelHistogram has length 256, and a GrayImage has 8 bits per pixel
        let cdf = unsafe { *hist.get_unchecked(*p as usize) as f32 };
        *p = (f32::min(max, max * (cdf - min) / (total - min))) as u8;
    });
}

type CumulativeHistogramRgb = [[usize; 65536]; 3];

pub fn histogram_rgb(image: &InputImage) -> CumulativeHistogramRgb {
    let mut hist = [[0_usize; 65536]; 3];

    for p in image.pixels() {
        for (channel, value) in p.0.iter().enumerate() {
            hist[channel][*value as usize] += 1;
        }
    }

    hist
}

pub fn find_cutoff_value(
    reverse: bool,
    channel_hist: [usize; 65536],
    (cutoff_min, cutoff_max): (f32, f32),
    debug: bool,
) -> u16 {
    let pixels_total = channel_hist.iter().sum::<usize>() as f32;

    let hist_iter: Box<dyn Iterator<Item = _>> = if reverse {
        Box::new(channel_hist.iter().rev())
    } else {
        Box::new(channel_hist.iter())
    };

    let mut pixels_count = 0 as f32;
    let mut candidates: Vec<(u16, f64)> = vec![];
    for (value, &freq) in hist_iter.enumerate() {
        pixels_count += freq as f32;
        let pixels_percentage = pixels_count / pixels_total;
        if pixels_percentage < cutoff_min {
            continue;
        } else if pixels_percentage >= cutoff_min && pixels_percentage <= cutoff_max {
            if freq > 0 {
                candidates.push((value as u16, freq as f64));
            }
        } else {
            break;
        }
    }

    if debug {
        debug_candidates(&candidates);
    }

    let minima = find_minima(&candidates);

    if reverse {
        u16::MAX - minima.unwrap_or(candidates[0].0)
    } else {
        minima.unwrap_or(candidates[0].0)
    }
}

pub fn stretch_channels_mut(image: &mut InputImage, clip_range: (f32, f32), debug: bool) {
    let hist = histogram_rgb(image);

    if debug {
        debug_histogram(&hist);
    }

    let min: Vec<f64> = (0..3)
        .map(|channel| find_cutoff_value(false, hist[channel], clip_range, debug) as f64)
        .collect();

    let max: Vec<f64> = (0..3)
        .map(|channel| find_cutoff_value(true, hist[channel], clip_range, debug) as f64)
        .collect();

    image.par_pixels_mut().for_each(|pixel| {
        for (channel, value) in pixel.0.iter_mut().enumerate() {
            *value = f64::min(
                u16::MAX as f64,
                u16::MAX as f64 * ((*value as f64 - min[channel]) / (max[channel] - min[channel])),
            ) as u16;
        }
    });

    let new_hist = histogram_rgb(image);

    debug_histogram(&new_hist);
}

fn find_minima(candidates: &Vec<(u16, f64)>) -> Option<u16> {
    let coeffs = polyfit::estimate(
        &DVector::from_iterator(candidates.len(), candidates.iter().map(|c| c.0 as f64)),
        &DVector::from_iterator(candidates.len(), candidates.iter().map(|c| c.1)),
        5,
    );

    for i in 2..candidates.len() {
        let prev = polyfit::evaluate(&coeffs, candidates[i - 2].0 as f64);
        let curr = polyfit::evaluate(&coeffs, candidates[i - 1].0 as f64);
        let next = polyfit::evaluate(&coeffs, candidates[i].0 as f64);

        if prev > curr && next > curr {
            return Some(candidates[i - 1].0);
        }
    }

    None
}

fn debug_histogram(hist: &CumulativeHistogramRgb) {
    for (channel, channel_hist) in hist.iter().enumerate() {
        let fname = format!(
            "{}_{}_hist.txt",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros(),
            channel
        );
        let mut file = std::fs::File::create(fname).unwrap();
        for (value, &count) in channel_hist.iter().enumerate() {
            if count > 0 {
                file.write(format!("{}\t{}\n", value, count).as_bytes())
                    .unwrap();
            }
        }
    }
}

fn debug_candidates(candidates: &Vec<(u16, f64)>) {
    let fname = format!(
        "{}.txt",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
    );

    let coeffs = polyfit::estimate(
        &DVector::from_iterator(candidates.len(), candidates.iter().map(|c| c.0 as f64)),
        &DVector::from_iterator(candidates.len(), candidates.iter().map(|c| c.1)),
        5,
    );

    let mut file = std::fs::File::create(fname).unwrap();
    for &(value, count) in candidates.iter() {
        let estimate = polyfit::evaluate(&coeffs, value as f64);
        file.write(format!("{}\t{}\t{}\n", value, count, estimate).as_bytes())
            .unwrap();
    }
}
