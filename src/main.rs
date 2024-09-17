// use las_rs::LasProcessor;

use las_trimmer::errors::MyError;
use std::fs;

use las::Point;
use las_trimmer::LasProcessor;

fn main() -> Result<(), MyError> {
    let folder_path = "\\\\file\\Research\\LidarPowerline\\03_RESEARCH\\03_POTREE_STREAM_WALK\\01_COCKLE_BAY\\02_LAS_TILES\\NW";
    // let output_path = format!("{}/test.laz", folder_path);
    let output_path = "C:/temp/NW.las".to_string();
    // laz is 6x slower than las
    let paths: Vec<String> = fs::read_dir(folder_path)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "las"))
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect();
    let processor = LasProcessor::new(paths, output_path, |_point: &Point| true);

    processor.process_lidar_files()?;

    Ok(())
}

// Point { x: 1820633.308, y: 5649529.221, z: 2727.763, intensity: 0, return_number: 1, number_of_returns: 1, scan_direction: RightToLeft, is_edge_of_flight_line: false, classification: Ground, is_synthetic: false, is_key_point: false, is_withheld: false, is_overlap: false, scanner_channel: 0, scan_angle: -8.922, user_data: 0, point_source_id: 1, gps_time: Some(165515.06842630001), color: Some(Color { red: 34695, green: 35980, blue: 39578 }), waveform: None, nir: None, extra_bytes: [74, 0, 204, 248, 1, 0] }
// Ok(Point { x: 820633308, y: 649529221, z: 2727763, intensity: 0, flags: ThreeByte(17, 0, 2), scan_angle: Scaled(-1487), user_data: 0, point_source_id: 1, gps_time: Some(165515.06842630001), color: Some(Color { red: 34695, green: 35980, blue: 39578 }), waveform: None, nir: None, extra_bytes: [74, 0, 204, 248, 1, 0] })
// Point { x: 1824400.6269999999, y: 5646603.57075, z: 1740.9005, intensity: 43427, return_number: 1, number_of_returns: 1, scan_direction: RightToLeft, is_edge_of_flight_line: false, classification: CreatedNeverClassified, is_synthetic: false, is_key_point: false, is_withheld: false, is_overlap: false, scanner_channel: 0, scan_angle: -10.458, user_data: 0, point_source_id: 0, gps_time: Some(304982501.4018127), color: Some(Color { red: 2827, green: 2827, blue: 4626 }), waveform: None, nir: None, extra_bytes: [] }
// Ok(Point { x: 12497554, y: -9965488, z: -1619272, intensity: 43427, flags: ThreeByte(17, 0, 0), scan_angle: Scaled(-1743), user_data: 0, point_source_id: 0, gps_time: Some(304982501.4018127), color: Some(Color { red: 2827, green: 2827, blue: 4626 }), waveform: None, nir: None, extra_bytes: [] })
