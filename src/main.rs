// use las_rs::LasProcessor;

use las_trimmer::errors::MyError;

use las::Point;
use las_trimmer::LasProcessor;

fn main() -> Result<(), MyError> {
    let paths = vec![
        "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/Mt_Ruapehu_Record26_31.laz".to_string(),
        "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/Mt_Ruapehu_Record20_25.laz".to_string(),
        "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/Mt_Ruapehu_Record14_19.laz".to_string(),
    ];
    let output_path =
        "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/test.laz".to_string();

    let processor = LasProcessor::new(paths, output_path, |point: &Point| {
        point.x > 1821710.0 && point.x < 1825753.0 && point.y > 5645723.0 && point.y < 5650440.0
    });

    processor.process_lidar_files()?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use las::Point;
    use las::Read;
    use las::Reader;
    use las::Writer;
    use las_trimmer::process_points::process_points;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test_process_points() {
        // Setup
        let mut reader = Reader::from_path("test_data.laz").unwrap();
        let writer = Arc::new(Mutex::new(
            Writer::from_path("test_output.laz", reader.header().clone()).unwrap(),
        ));
        let mut points_vec = Vec::<Point>::with_capacity(10000);
        let points_read = Arc::new(Mutex::new(0));
        let mut points_written = Mutex::new(0);
        let points_per_cycle = 10000;
        let vec_size = 10000;

        // Call the function
        let result = process_points(
            &mut reader,
            &mut Arc::clone(&writer),
            &mut points_vec,
            &points_read,
            &mut points_written,
            points_per_cycle,
            vec_size,
            |_| true,
        );

        // Check the result
        assert!(result.is_ok());

        // Check the output
        let points_written = points_written.into_inner().unwrap();
        assert_eq!(points_written, 10000);
    }
}

// Point { x: 1820633.308, y: 5649529.221, z: 2727.763, intensity: 0, return_number: 1, number_of_returns: 1, scan_direction: RightToLeft, is_edge_of_flight_line: false, classification: Ground, is_synthetic: false, is_key_point: false, is_withheld: false, is_overlap: false, scanner_channel: 0, scan_angle: -8.922, user_data: 0, point_source_id: 1, gps_time: Some(165515.06842630001), color: Some(Color { red: 34695, green: 35980, blue: 39578 }), waveform: None, nir: None, extra_bytes: [74, 0, 204, 248, 1, 0] }
// Ok(Point { x: 820633308, y: 649529221, z: 2727763, intensity: 0, flags: ThreeByte(17, 0, 2), scan_angle: Scaled(-1487), user_data: 0, point_source_id: 1, gps_time: Some(165515.06842630001), color: Some(Color { red: 34695, green: 35980, blue: 39578 }), waveform: None, nir: None, extra_bytes: [74, 0, 204, 248, 1, 0] })
// Point { x: 1824400.6269999999, y: 5646603.57075, z: 1740.9005, intensity: 43427, return_number: 1, number_of_returns: 1, scan_direction: RightToLeft, is_edge_of_flight_line: false, classification: CreatedNeverClassified, is_synthetic: false, is_key_point: false, is_withheld: false, is_overlap: false, scanner_channel: 0, scan_angle: -10.458, user_data: 0, point_source_id: 0, gps_time: Some(304982501.4018127), color: Some(Color { red: 2827, green: 2827, blue: 4626 }), waveform: None, nir: None, extra_bytes: [] }
// Ok(Point { x: 12497554, y: -9965488, z: -1619272, intensity: 43427, flags: ThreeByte(17, 0, 0), scan_angle: Scaled(-1743), user_data: 0, point_source_id: 0, gps_time: Some(304982501.4018127), color: Some(Color { red: 2827, green: 2827, blue: 4626 }), waveform: None, nir: None, extra_bytes: [] })
