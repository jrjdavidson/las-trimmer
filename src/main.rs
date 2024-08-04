// use las::point::Classification;
use las::Point;
use las::Read;
use las::Reader;
use las::Write;
use las::Writer;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;
use std::time::Instant;

fn main() {
    let vec_size = 100000;

    let _path = "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/merged_1.laz";

    let path2 = "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/Mt_Ruapehu_Record26_31.laz";
    let path3 = "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/Mt_Ruapehu_Record20_25.laz";
    let path4 = "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/Mt_Ruapehu_Record14_19.laz";
    let paths = vec![path2, path3, path4];

    let num_threads = num_cpus::get();
    println!("Number of logical cores is {}", num_threads);

    //keep track of points_written

    let total_points = Arc::new(Mutex::new(0));
    let total_points_clone = Arc::clone(&total_points);

    let points_written = Arc::new(Mutex::new(0));
    let points_written_clone = Arc::clone(&points_written);
    let points_read = Arc::new(Mutex::new(0));
    let points_read_clone = Arc::clone(&points_read);
    thread::spawn(move || loop {
        let start = Instant::now();
        thread::sleep(Duration::from_secs(1));
        {
            let mut points = points_written_clone.lock().unwrap();
            let mut points_r = points_read_clone.lock().unwrap();
            if *points_r == 0 {
                println!("No points were written in the last minute.");
                *points_r = 0;
                continue;
            }
            let mut total_points = total_points_clone.lock().unwrap();
            *total_points -= *points_r;
            println!(
                "Points written/read/left in the last minute: {}/{}/{}",
                *points, *points_r, *total_points
            );
            let time_elapsed = start.elapsed().as_secs();
            let points_per_second = *points_r / time_elapsed;
            let time_left_seconds = *total_points / points_per_second;
            let hours = time_left_seconds / 3600;
            let minutes = (time_left_seconds % 3600) / 60;
            let seconds = time_left_seconds % 60;
            println!(
                "Points per second: {}, Time left: {:02}:{:02}:{:02}",
                points_per_second, hours, minutes, seconds
            );

            *points = 0;
            *points_r = 0;
        }
    });
    // let transform = reader2.header().transforms().clone();
    // let header = reader1.header().point_format();
    // spawn threads
    let reader1 = Reader::from_path(paths[0]).unwrap();

    let writer = Arc::new(Mutex::new(
        Writer::from_path(
            "//file/Shared/SEESPhotoDatabase/Private/Pedro/01_Mt_Ruapehu_Lidar/test.laz",
            reader1.header().clone(),
        )
        .unwrap(),
    ));

    let paths: Vec<_> = paths.into_iter().collect();
    let mut handles = vec![];
    let mut readers = vec![];
    for path in paths {
        let reader = Reader::from_path(path).unwrap();
        let number_of_points = reader.header().number_of_points();
        let mut total_points = total_points.lock().unwrap();

        *total_points += number_of_points;
        println!("New Total:{:?}", total_points);
        readers.push(Arc::new(Mutex::new(reader)));
    }

    for i in 0..num_threads {
        let reader = Arc::clone(&readers[i % readers.len()]); // This will cycle through the paths if there are more CPUs than paths
        let writer = Arc::clone(&writer);
        let mut points_read = Arc::clone(&points_read);
        let mut points_written = Arc::clone(&points_written);
        let handle = thread::spawn(move || {
            let mut points_vec = Vec::<Point>::with_capacity(vec_size);

            process_points(
                reader.lock().unwrap(),
                writer.lock().unwrap(),
                &mut points_vec,
                &mut points_read,
                &mut points_written,
            );
        });

        handles.push(handle);
    }

    // Join the threads
    for handle in handles {
        handle.join().unwrap();
    }
    // for wrapped_point in reader1.points() {
    //     let point = wrapped_point.unwrap();
    //     let cl = point.classification;
    //     {
    //         let mut points = points_read.lock().unwrap();
    //         *points += 1;
    //     }

    //     if cl == Classification::Ground || cl == Classification::Unclassified {
    //         let result = writer.write(point);
    //         // let mut raw_point = point.into_raw(&transform).unwrap();
    //         // raw_point.extra_bytes = vec![];
    //         // let new_p = Point::new(raw_point, &transform);
    //         // // println!("{:?}", new_p);

    //         // let result = writer.write(new_p);

    //         match result {
    //             Ok(_) => {
    //                 let mut points_r = points_written.lock().unwrap();
    //                 *points_r += 1;
    //             }
    //             Err(e) => println!("{:?}", e),
    //         }
    //     }
    // }
}

fn process_points<W: std::io::Write + std::io::Seek + std::fmt::Debug + std::marker::Send>(
    mut reader: MutexGuard<Reader>,
    mut writer: MutexGuard<Writer<W>>,
    vec: &mut Vec<Point>,
    points_read: &Arc<Mutex<u64>>,
    points_written: &Mutex<i32>,
) {
    loop {
        let points_read_from_reader = reader.read_n_into(100000, vec).unwrap();
        // Break the loop if no more points were read
        if points_read_from_reader == 0 {
            break;
        }
        // Increment points_read by the number of points read from the reader
        {
            let mut points = points_read.lock().unwrap();
            *points += points_read_from_reader as u64;
        }
        while let Some(point) = vec.pop() {
            if point.x > 1821710.0
                && point.x < 1825753.0
                && point.y > 5645723.0
                && point.y < 5650440.0
            {
                let result = writer.write(point);
                match result {
                    Ok(_) => {
                        let mut points_w = points_written.lock().unwrap();
                        *points_w += 1;
                    }
                    Err(e) => println!("{:?}", e),
                }
            }
        }
    }
}

// Point { x: 1820633.308, y: 5649529.221, z: 2727.763, intensity: 0, return_number: 1, number_of_returns: 1, scan_direction: RightToLeft, is_edge_of_flight_line: false, classification: Ground, is_synthetic: false, is_key_point: false, is_withheld: false, is_overlap: false, scanner_channel: 0, scan_angle: -8.922, user_data: 0, point_source_id: 1, gps_time: Some(165515.06842630001), color: Some(Color { red: 34695, green: 35980, blue: 39578 }), waveform: None, nir: None, extra_bytes: [74, 0, 204, 248, 1, 0] }
// Ok(Point { x: 820633308, y: 649529221, z: 2727763, intensity: 0, flags: ThreeByte(17, 0, 2), scan_angle: Scaled(-1487), user_data: 0, point_source_id: 1, gps_time: Some(165515.06842630001), color: Some(Color { red: 34695, green: 35980, blue: 39578 }), waveform: None, nir: None, extra_bytes: [74, 0, 204, 248, 1, 0] })
// Point { x: 1824400.6269999999, y: 5646603.57075, z: 1740.9005, intensity: 43427, return_number: 1, number_of_returns: 1, scan_direction: RightToLeft, is_edge_of_flight_line: false, classification: CreatedNeverClassified, is_synthetic: false, is_key_point: false, is_withheld: false, is_overlap: false, scanner_channel: 0, scan_angle: -10.458, user_data: 0, point_source_id: 0, gps_time: Some(304982501.4018127), color: Some(Color { red: 2827, green: 2827, blue: 4626 }), waveform: None, nir: None, extra_bytes: [] }
// Ok(Point { x: 12497554, y: -9965488, z: -1619272, intensity: 43427, flags: ThreeByte(17, 0, 0), scan_angle: Scaled(-1743), user_data: 0, point_source_id: 0, gps_time: Some(304982501.4018127), color: Some(Color { red: 2827, green: 2827, blue: 4626 }), waveform: None, nir: None, extra_bytes: [] })
