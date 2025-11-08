use std::{io::Write, usize};

use image::GrayImage;
use imageproc::stats::cumulative_histogram;
use rayon::prelude::*;

use crate::conversion::InputImage;

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

type HistogramRgb = Vec<Vec<usize>>;

pub fn histogram_rgb(image: &InputImage, bins: usize) -> HistogramRgb {
    let mut hist = vec![vec![0; bins]; 3];

    for p in image.pixels() {
        for (channel, &value) in p.0.iter().enumerate() {
            let bin = (value as f32 / u16::MAX as f32) * (bins - 1) as f32;
            hist[channel][bin as usize] += 1;
        }
    }

    hist
}

pub fn find_cutoff_value(
    reverse: bool,
    channel_hist: &Vec<usize>,
    max_pixels_pct: f32,
    max_pixels_pct_diff: f32,
) -> u16 {
    let hard_pixels_pct_cutoff = 0.005;

    let hist_iter: Box<dyn Iterator<Item = _>> = if reverse {
        Box::new(channel_hist.iter().rev())
    } else {
        Box::new(channel_hist.iter())
    };

    let pixels_total = channel_hist.iter().sum::<usize>() as f32;

    let mut pixels_running_count = 0 as f32;
    let mut prev_count: usize = usize::MAX;
    let mut cutoff_value_by_count = None;
    let mut cutoff_value_by_diff = None;

    for (value, &count) in hist_iter.enumerate() {
        if count == 0 {
            continue;
        }

        pixels_running_count += count as f32;

        let pixels_pct = pixels_running_count / pixels_total;
        let pixels_diff_pct = (count - prev_count) as f32 / pixels_total;

        if pixels_pct > hard_pixels_pct_cutoff {
            break;
        }

        if cutoff_value_by_count.is_none() && pixels_pct > max_pixels_pct {
            cutoff_value_by_count = Some(value);
        }

        if cutoff_value_by_diff.is_none() && pixels_diff_pct > max_pixels_pct_diff {
            cutoff_value_by_diff = Some(value);
        }

        if cutoff_value_by_count.is_some() && cutoff_value_by_diff.is_some() {
            break;
        }

        prev_count = count;
    }

    let unscaled_cutoff_value = usize::max(
        cutoff_value_by_count.unwrap_or(0),
        cutoff_value_by_diff.unwrap_or(0),
    ) as f32;

    let max_value = (channel_hist.len() - 1) as f32;
    let cutoff_value = (u16::MAX as f32 * unscaled_cutoff_value / max_value) as u16;

    if reverse {
        u16::MAX - cutoff_value
    } else {
        cutoff_value
    }
}

pub fn stretch_channels_mut(image: &mut InputImage) {
    let max_pixels_pct = 0.0001;
    let max_pixels_pct_diff = 0.00005;

    // First, do a conservative stretch to ensure we use most of the value range in the histogram.
    let hist = histogram_rgb(&image, 65_536);
    let min: Vec<f64> = (0..3)
        .map(|channel| find_cutoff_value(false, &hist[channel], max_pixels_pct / 10.0, 0.0) as f64)
        .collect();
    let max: Vec<f64> = (0..3)
        .map(|channel| find_cutoff_value(true, &hist[channel], max_pixels_pct / 10.0, 0.0) as f64)
        .collect();
    image.par_pixels_mut().for_each(|pixel| {
        for (channel, value) in pixel.0.iter_mut().enumerate() {
            *value = f64::min(
                u16::MAX as f64,
                u16::MAX as f64 * ((*value as f64 - min[channel]) / (max[channel] - min[channel])),
            ) as u16;
        }
    });

    // Then, do one more pass to refine black and white levels.
    // Using a 256-bin histogram is an easy way to smooth out the histogram curve.
    let hist = histogram_rgb(&image, 256);
    let min: Vec<f64> = (0..3)
        .map(|channel| {
            find_cutoff_value(false, &hist[channel], max_pixels_pct, max_pixels_pct_diff) as f64
        })
        .collect();
    let max: Vec<f64> = (0..3)
        .map(|channel| {
            find_cutoff_value(true, &hist[channel], max_pixels_pct, max_pixels_pct_diff) as f64
        })
        .collect();
    image.par_pixels_mut().for_each(|pixel| {
        for (channel, value) in pixel.0.iter_mut().enumerate() {
            *value = f64::min(
                u16::MAX as f64,
                u16::MAX as f64 * ((*value as f64 - min[channel]) / (max[channel] - min[channel])),
            ) as u16;
        }
    });

    histogram_rgb(&image, 256);
}

#[allow(dead_code)]
fn debug_histogram(hist: &Vec<Vec<usize>>) {
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
