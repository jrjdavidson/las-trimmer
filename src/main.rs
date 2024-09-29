use las_trimmer::errors::MyError;
use std::fs;
use std::path::PathBuf;

use las::Point;
use las_trimmer::LasProcessor;

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "Las file trimmer")]
#[command(version = "0.1.0")]
#[command(about = "Reads las and laz files and optionally trims some points.", long_about = None)]
struct Cli {
    /// Sets the input file or folder
    #[arg(short, long, value_name = "INPUT")]
    input: PathBuf,

    /// Sets the output file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,

    #[arg(short, long, value_name = "Strip extra bytes")]
    strip_extra_bytes: bool,

    /// Function selection flag
    #[arg(short, long, value_name = "FUNCTION", value_enum)]
    function: Option<Function>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Function {
    AlwaysTrue,
    AlwaysFalse,
}

fn retu(_point: &Point) -> bool {
    // Define your filter logic here
    true
}

fn filter_function_2(_point: &Point) -> bool {
    // Define your filter logic here
    false
}

fn main() -> Result<(), MyError> {
    let cli = Cli::parse();

    let input_path = cli.input;
    let output_path = cli.output;
    let strip_extra_bytes = cli.strip_extra_bytes;
    let function = cli.function;

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

    let filter_function: fn(&Point) -> bool = match function {
        Some(Function::AlwaysTrue) => retu,
        Some(Function::AlwaysFalse) => filter_function_2,
        None => retu,
    };

    let processor = LasProcessor::new(
        paths,
        output_path.to_string_lossy().to_string(),
        filter_function,
        strip_extra_bytes,
    );

    processor.process_lidar_files()?;

    Ok(())
}
