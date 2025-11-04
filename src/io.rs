use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

use image::{DynamicImage, EncodableLayout, ImageBuffer, ImageError, Pixel, PixelWithColorType};

pub fn save_image<'a, P, Container>(
    path: &str,
    suffix: &str,
    extension: &str,
    image: ImageBuffer<P, Container>,
) -> Result<(), ImageError>
where
    P: Pixel + PixelWithColorType,
    [P::Subpixel]: EncodableLayout,
    Container: Deref<Target = [P::Subpixel]>,
    ImageBuffer<P, Container>: Into<DynamicImage>,
{
    let output_path = format!("{}.{}.{}", path, suffix, extension);

    if image.save(&output_path).is_ok() {
        println!("Saved (8-bit rgb) {}", output_path);
        return Ok(());
    }

    Into::<DynamicImage>::into(image)
        .to_rgb8()
        .save(&output_path)?;
    println!("Saved {}", output_path);
    Ok(())
}

pub fn read_dir_raw_files<'a>(dir: &'a str) -> std::io::Result<Vec<PathBuf>> {
    let path = Path::new(dir);

    if !path.is_dir() {
        return Ok(vec![]);
    }

    let dir_entries = fs::read_dir(dir)?.flatten();

    let raw_file_paths = dir_entries.flat_map(|dir_entry| {
        if has_raw_file_extension(&dir_entry.path()) {
            Some(dir_entry.path())
        } else {
            None
        }
    });

    Ok(raw_file_paths.collect())
}

pub fn has_raw_file_extension(path: &Path) -> bool {
    if path.is_file()
        && let Some(ext) = path.extension()
    {
        [
            "3fr", "ari", "arw", "bay", "braw", "cap", "cr2", "cr3", "cri", "crw",
            "dcr", "dcs", "dng", "dng", "drf", "eip", "erf", "fff", "gpr", "iiq", "jxs",
            "k25", "kdc", "mdc", "mef", "mos", "mrw", "nef", "nrw", "orf", "pef", "ptx",
            "pxn", "r3d", "raf", "raw", "raw", "rw2", "rwl", "rwz", "sr2", "srf", "srw",
            "tco", "x3f",
        ]
        .contains(
            &ext.to_ascii_lowercase()
                .to_str()
                .expect("file extension should be a valid UTF-8 sequence"),
        )
    } else {
        false
    }
}
