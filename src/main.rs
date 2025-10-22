use clap::Parser;
use yancy::raw_processor;

/// Yet Another Negative Conversion thingY
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of the file to convert
    #[arg(short, long)]
    path: String,
}

fn main() {
    let args = Args::parse();

    println!("Converting file {}...", args.path);

    // Load the RAW file
    match raw_processor::load_raw_image(&args.path) {
        Ok(image) => {
            println!(
                "Successfully loaded RAW image: {}x{} pixels",
                image.width(),
                image.height()
            );
            // TODO: Apply negative inversion and save the result
            // For now, just save the processed image
            let output_path = format!("{}.png", args.path);
            if let Err(e) = image.save(&output_path) {
                eprintln!("Failed to save image: {}", e);
                std::process::exit(1);
            }
            println!("Saved to {}", output_path);
        }
        Err(e) => {
            eprintln!("Failed to load RAW image: {}", e);
            std::process::exit(1);
        }
    }
}