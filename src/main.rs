use std::path::Path;

use clap::{Args, Parser};
use yancy::{conversion, io, raw_processor};

/// yet another negative conversion thingy
#[derive(Parser, Debug)]
#[command(version, about, long_about)]
struct Cli {
    #[command(flatten)]
    input: Input,

    /// output file format
    #[arg(long, default_value = "tiff")]
    output_format: String,

    /// output file suffix
    #[arg(long, default_value = "positive")]
    output_suffix: String,

    #[arg(long, default_value_t = false)]
    half_frame: bool,

    /// saves intermediate images during processing
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct Input {
    /// Path of the file(s) to convert
    #[arg(short = 'f', long, value_delimiter = ' ', num_args = 1..)]
    file: Option<Vec<String>>,

    /// Directory of files to convert
    #[arg(short = 'd', long)]
    dir: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let files: Vec<String> = if let Some(files) = &args.input.file {
        files
            .into_iter()
            .flat_map(|file| {
                if io::has_raw_file_extension(&Path::new(&file)) {
                    Some(String::from(file))
                } else {
                    None
                }
            })
            .collect()
    } else if let Some(dir) = &args.input.dir {
        io::read_dir_raw_files(&dir)?
            .iter()
            .map(|file_path| {
                String::from(
                    file_path
                        .to_str()
                        .expect("file path should be a valid UTF-8 sequence"),
                )
            })
            .collect()
    } else {
        panic!("expected either directory or file inputs");
    };

    files.into_iter().for_each(|file| {
        if let Err(e) = process_file(&file, &args) {
            println!("Unable to process file {}: {}", file, e);
        }
    });

    Ok(())
}

fn process_file(file: &str, args: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    println!("Converting file {}...", file);

    let image = raw_processor::load_raw_image(&file)?;

    if args.debug {
        println!(
            "Successfully loaded RAW image: {}x{} pixels",
            image.width(),
            image.height()
        );

        io::save_image(&file, "original", "jpeg", image.clone())?;
    }

    let debug_file_path = if args.debug { Some(file) } else { None };

    if !args.half_frame {
        let converted = conversion::convert(&image, debug_file_path)?;
        io::save_image(&file, &args.output_suffix, &args.output_format, converted)?;
    } else {
        let halves = conversion::split_image(image);
        for (image, half_suffix) in halves.into_iter().zip('a'..='b') {
            let converted = conversion::convert(&image, debug_file_path)?;
            let suffix = format!("{}.{}", args.output_suffix, half_suffix);
            io::save_image(&file, &suffix, &args.output_format, converted)?;
        }
    }

    Ok(())
}
