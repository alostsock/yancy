use clap::Parser;
use yancy::{io, raw_processor, conversion};

/// Yet Another Negative Conversion thingY
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of the file to convert
    #[arg(short, long)]
    file: String,

    /// Scales images to half their width and height
    #[arg(short = 's', long, default_value_t = false)]
    half_size: bool,

    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    println!("Converting file {}...", args.file);

    let image = match raw_processor::load_raw_image(&args.file, args.half_size) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("Failed to load RAW image: {}", e);
            std::process::exit(1);
        }
    };

    if args.debug {
        println!(
            "Successfully loaded RAW image: {}x{} pixels",
            image.width(),
            image.height()
        );

        io::save_image(&args.file, "original", "tiff", image.clone());
    }

    let debug_file_path = if args.debug { Some(args.file.as_str()) } else { None };

    let converted = conversion::convert(&image, debug_file_path);

    io::save_image(&args.file, "positive", "tiff", converted);
}
