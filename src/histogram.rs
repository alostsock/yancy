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

pub fn find_cutoff_value(channel_hist: [usize; 65536], cutoff: f32) -> u16 {
    let mut count = 0 as f32;
    let mut current_value = 0_u16;
    let total = channel_hist.iter().sum::<usize>() as f32;

    for (value, &freq) in channel_hist.iter().enumerate() {
        count += freq as f32;
        current_value = value as u16;
        if (count / total) > cutoff {
            break;
        }
    }

    current_value
}

pub fn find_cutoff_value_rev(channel_hist: [usize; 65536], cutoff: f32) -> u16 {
    let mut count = 0 as f32;
    let mut current_value = 0_u16;
    let total = channel_hist.iter().sum::<usize>() as f32;

    for (value, &freq) in channel_hist.iter().rev().enumerate() {
        count += freq as f32;
        current_value = value as u16;
        if (count / total) > cutoff {
            break;
        }
    }

    u16::MAX - current_value
}

pub fn stretch_channels_mut(image: &mut InputImage, cutoff: f32) {
    let hist = histogram_rgb(image);

    let min = [
        find_cutoff_value(hist[0], cutoff) as f32,
        find_cutoff_value(hist[1], cutoff) as f32,
        find_cutoff_value(hist[2], cutoff) as f32,
    ];
    let max = [
        find_cutoff_value_rev(hist[0], cutoff) as f32,
        find_cutoff_value_rev(hist[1], cutoff) as f32,
        find_cutoff_value_rev(hist[2], cutoff) as f32,
    ];

    image.par_pixels_mut().for_each(|pixel| {
        for (channel, value) in pixel.0.iter_mut().enumerate() {
            *value = f32::min(
                u16::MAX as f32,
                u16::MAX as f32 * ((*value as f32 - min[channel]) / (max[channel] - min[channel])),
            ) as u16;
        }
    });
}
