use clap::Subcommand;
use las_trimmer::errors::MyError;
use std::fs;
use std::path::PathBuf;

use las::Point;
use las_trimmer::LasProcessor;

use clap::Parser;
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
    input: PathBuf,

    /// Sets the output file. File type must be either .las or .laz
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,

    /// Strips extra bytes from the LAS/LAZ file. Can dramatically decrease resulting size
    #[arg(short, long, value_name = "Strip extra bytes")]
    strip_extra_bytes: bool,
    /// Specifies the filtering function to apply to points.
    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    /// Default 'filter' function- returns and writes all points.
    AlwaysTrue,
    /// Only reads points and doesn't write any- for testing purposes mainly.
    AlwaysFalse,
    /// Crops the point cloud based on specified bounds. Max values are excluded (up to but not including max_x), min values are included (up to and including min_x)
    Crop {
        /// Minimum x value
        #[arg(long, allow_hyphen_values = true, value_name = "MIN_X")]
        min_x: Option<f64>,

        /// Maximum x value
        #[arg(long, allow_hyphen_values = true, value_name = "MAX_X")]
        max_x: Option<f64>,

        /// Minimum y value
        #[arg(long, allow_hyphen_values = true, value_name = "MIN_Y")]
        min_y: Option<f64>,

        /// Maximum y value
        #[arg(long, allow_hyphen_values = true, value_name = "MAX_Y")]
        max_y: Option<f64>,

        /// Minimum z value
        #[arg(long, allow_hyphen_values = true, value_name = "MIN_Z")]
        min_z: Option<f64>,

        /// Maximum z value
        #[arg(long, allow_hyphen_values = true, value_name = "MAX_Z")]
        max_z: Option<f64>,
    },
}

fn return_true(_point: &Point) -> bool {
    true
}

fn return_false(_point: &Point) -> bool {
    false
}
fn crop_filter(
    point: &Point,
    min_x: Option<f64>,
    max_x: Option<f64>,
    min_y: Option<f64>,
    max_y: Option<f64>,
    min_z: Option<f64>,
    max_z: Option<f64>,
) -> bool {
    if let Some(min_x) = min_x {
        if point.x < min_x {
            return false;
        }
    }
    if let Some(max_x) = max_x {
        if point.x >= max_x {
            return false;
        }
    }
    if let Some(min_y) = min_y {
        if point.y < min_y {
            return false;
        }
    }
    if let Some(max_y) = max_y {
        if point.y >= max_y {
            return false;
        }
    }
    if let Some(min_z) = min_z {
        if point.z < min_z {
            return false;
        }
    }
    if let Some(max_z) = max_z {
        if point.z >= max_z {
            return false;
        }
    }
    true
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

    let filter_function: Box<dyn Fn(&Point) -> bool + Send + Sync + 'static> = match &cli.command {
        Some(Commands::Crop {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        }) => {
            let min_x = *min_x;
            let max_x = *max_x;
            let min_y = *min_y;
            let max_y = *max_y;
            let min_z = *min_z;
            let max_z = *max_z;

            Box::new(move |point: &Point| {
                crop_filter(point, min_x, max_x, min_y, max_y, min_z, max_z)
            })
        }
        Some(Commands::AlwaysTrue) => Box::new(return_true),
        Some(Commands::AlwaysFalse) => Box::new(return_false),
        None => Box::new(return_true),
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
