#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("yancy/src/raw_processor.h");

        type RawProcessor;

        fn new_raw_processor() -> UniquePtr<RawProcessor>;

        fn open_and_process(self: Pin<&mut Self>, path: String);
        fn get_width(&self) -> u16;
        fn get_height(&self) -> u16;
        fn get_bits(&self) -> u16;
        fn get_data_size(&self) -> u32;
        #[allow(dead_code)]
        fn copy_data_to_buffer_u8(&self, buffer: &mut [u8]) -> Result<()>;
        fn copy_data_to_buffer_u16(&self, buffer: &mut [u16]) -> Result<()>;
    }
}

use std::path::Path;
use image::{ImageBuffer, Rgb};

pub fn load_raw_image<P: AsRef<Path>>(path: P) -> Result<ImageBuffer<Rgb<u16>, Vec<u16>>, Box<dyn std::error::Error>> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or("Invalid UTF-8 in path")?;

    let mut processor = ffi::new_raw_processor();

    processor.pin_mut().open_and_process(path_str.to_string());

    let width = processor.get_width();
    let height = processor.get_height();
    let bits = processor.get_bits();
    let data_size = processor.get_data_size(); // in bytes; = width * height * 3 channels * bits / 8

    let rgb_image = if bits == 16 {
        let mut buffer = vec![0u16; data_size as usize];
        processor.copy_data_to_buffer_u16(&mut buffer)?;

        ImageBuffer::from_raw(width as u32, height as u32, buffer)
            .ok_or("Failed to create image from raw buffer")?
    } else {
        return Err(format!("Unsupported bit depth: {}", bits).into());
    };

    Ok(rgb_image)
}
