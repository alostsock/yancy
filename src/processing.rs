use image::GrayImage;
use imageproc::stats::cumulative_histogram;
use rayon::prelude::*;

/// Unfortunately, imageproc's `equalize_histogram_mut` doesn't preserve black
/// or white levels. Here, we keep track of the `min` CDF value so that pixels
/// with values 0 and 255 remain 0 and 255, respectively.
pub fn normalize_histogram_mut(image: &mut GrayImage) {
    let hist = cumulative_histogram(image).channels[0];
    let min = hist[0] as f32;
    let total = hist[255] as f32;

    let iter = image.par_iter_mut();

    iter.for_each(|p| {
        // JUSTIFICATION
        //  Benefit
        //      Using checked indexing here makes this function take 1.1x longer, as measured
        //      by bench_equalize_histogram_mut
        //  Correctness
        //      Each channel of CumulativeChannelHistogram has length 256, and a GrayImage has 8 bits per pixel
        let cdf = unsafe { *hist.get_unchecked(*p as usize) as f32 };
        *p = (f32::min(255f32, 255f32 * (cdf - min) / (total - min))) as u8;
    });
}
