use las_trimmer::errors::MyError;
use std::fs;
use std::path::PathBuf;

use las::Point;
use las_trimmer::LasProcessor;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Sets the input file or folder
    #[arg(short, long, value_name = "INPUT")]
    input: PathBuf,

    /// Sets the output file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,

    #[arg(short, long, value_name = "Strip extra bytes")]
    strip_extra_bytes: bool,
}

fn main() -> Result<(), MyError> {
    let cli = Cli::parse();

    let input_path = cli.input;
    let output_path = cli.output;
    let strip_extra_bytes = cli.strip_extra_bytes;

    // Check if the output file has a valid extension
    let output_extension = output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    if output_extension != "las" && output_extension != "laz" {
        return Err(MyError::InvalidOutputExtension);
    }

    let paths = if input_path.is_file() {
        vec![input_path.to_string_lossy().to_string()]
    } else if input_path.is_dir() {
        fs::read_dir(input_path)?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "las"))
            .map(|entry| entry.path().to_string_lossy().to_string())
            .collect()
    } else {
        return Err(MyError::InvalidInputPath);
    };

    println!("{:?}", paths);

    let processor = LasProcessor::new(
        paths,
        output_path.to_string_lossy().to_string(),
        |_point: &Point| true,
        strip_extra_bytes,
    );

    processor.process_lidar_files()?;

    Ok(())
}
