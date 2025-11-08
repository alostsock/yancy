use std::path::Path;

use clap::{Args, Parser, ValueEnum};
use image::{ConvertColorOptions, metadata::Cicp};
use yancy::{conversion, io, raw_processor};

/// yet another negative conversion thingy
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    #[command(flatten)]
    input: Input,

    /// Output file format
    #[arg(long, default_value_t = OutputFormat::Tiff)]
    output_format: OutputFormat,

    /// Output file suffix
    #[arg(long, default_value = "positive")]
    output_suffix: String,

    /// Splits input file(s) in half vertically before processing
    #[arg(long, default_value_t = false)]
    half_frame: bool,

    /// The expected aspect ratio as width/height. Defaults to 1.5 (3:2 landscape), or 0.7083 (17:24 portrait) for half frame
    #[arg(long)]
    aspect_ratio: Option<f32>,

    /// Amount of additional crop after border removal, as a percentage of the original image's width and height
    #[arg(short = 'c', long, default_value_t = 0.01)]
    crop: f32,

    /// Saves intermediate images during processing
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

#[derive(Clone, ValueEnum, Debug)]
enum OutputFormat {
    Png,
    Jpeg,
    Webp,
    Tiff,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Png => write!(f, "png"),
            Self::Jpeg => write!(f, "jpeg"),
            Self::Webp => write!(f, "webp"),
            Self::Tiff => write!(f, "tiff"),
        }
    }
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

    let mut image = raw_processor::load_raw_image(&file)?;
    image.set_color_space(Cicp::SRGB_LINEAR)?;
    image.apply_color_space(Cicp::SRGB, ConvertColorOptions::default())?;

    if args.debug {
        println!(
            "Successfully loaded RAW image: {}x{} pixels",
            image.width(),
            image.height()
        );

        io::save_image(&file, "original", "jpeg", image.clone())?;
    }

    if !args.half_frame {
        let aspect_ratio = args.aspect_ratio.unwrap_or(1.5);
        let debug_file_path = if args.debug { Some(file) } else { None };
        let converted = conversion::convert(&image, aspect_ratio, args.crop, debug_file_path)?;
        io::save_image(
            &file,
            &args.output_suffix,
            &args.output_format.to_string(),
            converted,
        )?;
    } else {
        let halves = conversion::split_image(image);

        for (image, half_suffix) in halves.into_iter().zip('a'..='b') {
            let aspect_ratio = args.aspect_ratio.unwrap_or(0.7083);
            let file_half = format!("{}.{}", file, half_suffix);
            let debug_path = if args.debug {
                Some(file_half.as_str())
            } else {
                None
            };
            let converted = conversion::convert(&image, aspect_ratio, args.crop, debug_path)?;
            io::save_image(
                &file_half,
                &args.output_suffix,
                &args.output_format.to_string(),
                converted,
            )?;
        }
    }

    Ok(())
}
