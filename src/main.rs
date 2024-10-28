use clap::{Parser, ValueEnum};
use las::Point;
use las_trimmer::errors::MyError;
use las_trimmer::{LasProcessor, SharedFunction};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Las file trimmer
///
/// This tool reads LAS and LAZ files and optionally trims some points based on specified criteria. Using the excellent las-rs crate (https://docs.rs/las/latest/las/) that does most of the heavy lifting in this package.
#[derive(Parser)]
#[command(name = "Las file trimmer")]
#[command(version = "0.1.0")]
#[command(
    about = "Reads las and laz files and optionally trims/crops some points.",
    long_about = "This tool reads LAS and LAZ files and optionally trims some points based on specified criteria. Using the excellent las-rs crate (https://docs.rs/las/latest/las/) that does most of the heavy lifting."
)]
struct Cli {
    /// Sets the input file or folder
    #[arg(short, long, value_name = "INPUT")]
    input: Vec<PathBuf>,

    /// Sets the output files. File types must be either .las or .laz
    #[arg(short, long, value_name = "OUTPUTS")]
    output: Vec<PathBuf>,

    /// Strips extra bytes from the LAS/LAZ file. Can dramatically decrease resulting size
    #[arg(short, long, value_name = "Strip extra bytes")]
    strip_extra_bytes: bool,

    /// Specifies the filtering function to apply to points.
    #[arg(short, long, value_name = "FILTER")]
    filter: Vec<FilterType>,
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum FilterType {
    AlwaysTrue,
    AlwaysFalse,
}
fn return_true(_point: &Point) -> bool {
    true
}

fn return_false(_point: &Point) -> bool {
    false
}

fn main() -> Result<(), MyError> {
    let cli = Cli::parse();

    let input_paths = cli.input;
    let output_paths: Vec<String> = cli
        .output
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let strip_extra_bytes = cli.strip_extra_bytes;

    // Check if the output files have valid extensions
    for output_path in &output_paths {
        let path_buf = PathBuf::from(output_path);
        let output_extension = path_buf
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        if output_extension != "las" && output_extension != "laz" {
            return Err(MyError::InvalidOutputExtension);
        }
    }

    let mut paths = Vec::new();
    for input_path in input_paths {
        if input_path.is_file() {
            paths.push(input_path.to_string_lossy().to_string());
        } else if input_path.is_dir() {
            let dir_paths: Vec<String> = fs::read_dir(input_path)?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "las"))
                .map(|entry| entry.path().to_string_lossy().to_string())
                .collect();
            paths.extend(dir_paths);
        } else {
            return Err(MyError::InvalidInputPath);
        }
    }

    println!("{:?}", paths);

    let filter_functions: Vec<SharedFunction> = cli
        .filter
        .iter()
        .map(|filter| match filter {
            FilterType::AlwaysTrue => Arc::new(return_true) as SharedFunction,
            FilterType::AlwaysFalse => Arc::new(return_false) as SharedFunction,
        })
        .collect();

    // Check that the number of filter functions matches the number of output files
    if filter_functions.len() != output_paths.len() {
        return Err(MyError::MismatchedFiltersAndOutputs);
    }

    let processor = LasProcessor::new(paths, output_paths, filter_functions, strip_extra_bytes);

    processor.process_lidar_files()?;

    Ok(())
}
