/// `LasProcessor` is a struct that represents a processor for LiDAR files.
///
/// # Fields
///
/// * `paths`: A vector of strings representing the paths to the input LiDAR files.
/// * `output_path`: A string representing the path to the output LiDAR file.
/// * `condition`: An `Arc` containing a closure that takes a `Point` as input and returns a boolean. This closure is applied to each point read from the input files. Only points for which the closure returns `true` are written to the output file.
///
/// # Methods
///
/// * `new`: This method creates a new `LasProcessor`. It takes as input a vector of strings representing the paths to the input LiDAR files, a string representing the path to the output LiDAR file, and a closure that takes a `Point` as input and returns a boolean. It returns a `LasProcessor`.
///
/// * `process_lidar_files`: This method processes the LiDAR files. It reads points from the input files, applies the condition to each point, and writes the points that meet the condition to the output file. It returns a `Result<(), MyError>`. If the method completes successfully, it returns `Ok(())`. If an error occurs, it returns `Err(MyError)`.
///
/// # Example
///
/// ```rust
/// use las_trimmer::LasProcessor;
/// let processor = LasProcessor::new(
///     vec![
///         "tests/data/input1.las".to_string(),
///         "tests/data/input2.las".to_string(),
///     ],
///     "output.laz".to_string(),
///     |point| point.intensity > 20,
///     false
/// );
///
/// processor.process_lidar_files().unwrap();
/// ```
pub mod errors;
use crate::errors::MyError;
use las::Point;
use las::Reader;
use las::Writer;
use num_format::{Locale, ToFormattedString};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use threadpool::ThreadPool;

/// `LasProcessor` is a struct that represents a processor for LiDAR files.
pub struct LasProcessor {
    /// A vector of strings representing the paths to the input LiDAR files.
    paths: Vec<String>,
    /// A string representing the path to the output LiDAR file.
    output_path: String,
    /// An `Arc` containing a closure that takes a `Point` as input and returns a boolean. This closure is applied to each point read from the input files. Only points for which the closure returns `true` are written to the output file.
    condition: Arc<dyn Fn(&Point) -> bool + Send + Sync>,
    vec_size: u64,
    strip_extra_bytes: bool,
}
impl LasProcessor {
    /// This method creates a new `LasProcessor`. It takes as input a vector of strings representing the paths to the input LiDAR files, a string representing the path to the output LiDAR file, and a closure that takes a `las::Point` as input and returns a boolean. It returns a `LasProcessor`.
    pub fn new<F>(
        paths: Vec<String>,
        output_path: String,
        condition: F,
        strip_extra_bytes: bool,
    ) -> Self
    where
        F: Fn(&Point) -> bool + Send + Sync + 'static,
    {
        Self {
            paths,
            output_path,
            vec_size: 1000 as u64, // can modulate this value to see effect on speed?
            condition: Arc::new(condition),
            strip_extra_bytes,
        }
    }

    /// This method processes the LiDAR files. It reads points from the input files, applies the condition to each point, and writes the points that meet the condition to the output file. It returns a `Result<(), MyError>`. If the method completes successfully, it returns `Ok(())`. If an error occurs, it returns `Err(MyError)`.
    pub fn process_lidar_files(&self) -> Result<(), MyError> {
        let start = Instant::now();
        let number_locale = &Locale::en;

        let vec_size = self.vec_size.clone();
        let num_threads = num_cpus::get();
        println!("Number of logical cores is {}", num_threads);

        let total_points_to_read = Arc::new(Mutex::new(0));
        let total_points_to_read_clone = Arc::clone(&total_points_to_read);
        let total_points_to_write = Arc::new(Mutex::new(0));
        let total_points_to_write_clone = Arc::clone(&total_points_to_write);

        let points_written = Arc::new(Mutex::new(0));
        let points_written_clone = Arc::clone(&points_written);
        let points_read = Arc::new(Mutex::new(0));
        let points_read_clone = Arc::clone(&points_read);

        thread::spawn(move || -> Result<(), MyError> {
            let mut previous_read = 0;
            let mut previous_written = 0;
            loop {
                let start = Instant::now();
                let sleep_time = 1;
                thread::sleep(Duration::from_secs(sleep_time));
                {
                    let points_w = points_written_clone
                        .lock()
                        .map_err(|_| MyError::LockError)?;
                    let points_r = points_read_clone.lock().map_err(|_| MyError::LockError)?;
                    let time_elapsed = start.elapsed().as_secs();

                    if *points_r == 0 && *points_w == 0 {
                        println!(
                            "No points were written or read in the last {} second(s).",
                            { time_elapsed }
                        );
                        continue;
                    }
                    let total_points_to_read = total_points_to_read_clone
                        .lock()
                        .map_err(|_| MyError::LockError)?;
                    let points_to_read_left = *total_points_to_read - *points_r;
                    let total_points_to_write = total_points_to_write_clone
                        .lock()
                        .map_err(|_| MyError::LockError)?;
                    println!("{:?}", total_points_to_write);
                    println!("{:?}", points_w);

                    let points_to_write_left = *total_points_to_write - *points_w;

                    let percentage = (*points_r as f64 / *total_points_to_read as f64) * 100.0;
                    let read_in_last_interval = *points_r - previous_read;
                    let written_in_last_interval = *points_w - previous_written;
                    println!(
                        "Points read/written in the last {} second(s) and left to read/write : {} / {} / {} / {} / {:.2}%",
                        time_elapsed,
                        (read_in_last_interval).to_formatted_string(number_locale),
                        (written_in_last_interval).to_formatted_string(number_locale),
                        (points_to_read_left).to_formatted_string(number_locale),
                        (points_to_write_left).to_formatted_string(number_locale),
                        percentage
                    );
                    previous_read = *points_r;
                    previous_written = *points_w;
                }
            }
        });
        let header;
        use las::point::Format;
        use las::Builder;
        {
            let reader1 = Reader::from_path(&self.paths[0])?;
            let old_header = reader1.header().clone();
            if self.strip_extra_bytes {
                let format_u8 = old_header.point_format().to_u8()?;

                let mut new_format = Format::new(format_u8).unwrap();
                let mut builder = Builder::new(old_header.into_raw()?)?;
                new_format.extra_bytes = 0;
                builder.point_format = new_format;

                header = builder.into_header().unwrap();
            } else {
                header = old_header;
            }
        }

        let paths: Vec<_> = self.paths.iter().collect();
        let sendthreads = num_threads / 2;
        let (tx, rx) = mpsc::channel();
        let pool = ThreadPool::new(sendthreads);
        // Reader threads
        let total_paths = self.paths.len(); // Get the total number of paths
        let mut i = 0;
        for path in paths {
            i += 1;
            let path = path.clone();
            let tx = tx.clone();
            let condition = self.condition.clone();
            let points_read_clone = Arc::clone(&points_read);
            let total_points_to_read_clone = Arc::clone(&total_points_to_read);
            let total_points_to_write_clone = Arc::clone(&total_points_to_write);

            println!("Starting read thread {} for {:?}", i, path);
            {
                let reader = Reader::from_path(&path).unwrap();
                let number_of_points = reader.header().number_of_points();
                {
                    let mut total_points_to_read = total_points_to_read_clone
                        .lock()
                        .map_err(|_| MyError::LockError)
                        .unwrap();

                    *total_points_to_read += &number_of_points;
                    println!(
                        "{}/{}|| New Total:{}",
                        i,
                        total_paths,
                        total_points_to_read.to_formatted_string(number_locale)
                    );
                }
            }

            pool.execute(move || {
                let start_time = Instant::now(); // Start the timer

                let mut reader = Reader::from_path(&path).unwrap();
                let mut points_vec = Vec::with_capacity(vec_size as usize);
                let mut total_points_read = 0; // Track total points read
                for wrapped_point in reader.points() {
                    let point = wrapped_point.unwrap();
                    total_points_read += 1; // Update total points read

                    {
                        let mut points = points_read_clone
                            .lock()
                            .map_err(|_| MyError::LockError)
                            .unwrap();
                        *points += 1;
                    }

                    if condition(&point) {
                        points_vec.push(point);
                        if points_vec.len() >= vec_size.try_into().unwrap() {
                            {
                                let mut points_tw = total_points_to_write_clone
                                    .lock()
                                    .map_err(|_| MyError::LockError)
                                    .unwrap();
                                *points_tw += points_vec.len();
                            }
                            tx.send(points_vec.clone())
                                .map_err(|_| MyError::SendError)
                                .unwrap();
                            points_vec.clear(); // Clear the points_vec after sending
                        }
                    }
                }

                // Send any remaining points in the points_vec
                if !points_vec.is_empty() {
                    {
                        let mut points_tw = total_points_to_write_clone
                            .lock()
                            .map_err(|_| MyError::LockError)
                            .unwrap();
                        *points_tw += points_vec.len();
                    }
                    tx.send(points_vec).map_err(|_| MyError::SendError).unwrap();
                }

                let duration = start_time.elapsed(); // End the timer
                let points_per_second = total_points_read as f64 / duration.as_secs_f64();

                println!("Done : {:?} ({} out of {})", path, i, total_paths);
                println!("Size : {:?}", reader.header().number_of_points());
                println!("Total points read: {}", total_points_read);
                println!("Time taken: {:.2?}", duration);
                println!("Read speed: {:.2} points/second", points_per_second);
            });
        }
        drop(tx);
        // Single writer thread
        let writer_pwc = Arc::clone(&points_written);
        let output_path = self.output_path.clone();
        let mut writer = Writer::from_path(output_path, header)?;

        while let Ok(points_vec) = rx.recv() {
            let no_of_points = points_vec.len().clone();

            for mut point in points_vec {
                if self.strip_extra_bytes {
                    point.extra_bytes.clear();
                }
                writer.write_point(point)?;
            }
            {
                let mut points_w = writer_pwc.lock().map_err(|_| MyError::LockError)?;
                *points_w += no_of_points;
            }
        }

        let end_pwc = Arc::clone(&points_written);

        let end_prc = Arc::clone(&points_read);

        let points = end_pwc.lock().map_err(|_| MyError::LockError)?;
        let points_r = end_prc.lock().map_err(|_| MyError::LockError)?;

        println!(
            "Points written/read at the end of script: {}/{}",
            *points, *points_r
        );

        println!(
            "Total Points written {}",
            *(Arc::clone(&total_points_to_write)
                .lock()
                .map_err(|_| MyError::LockError)?)
        );

        let duration = start.elapsed();
        println!("Time taken: {:?}", duration);

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use las::{Builder, Point, Writer};
    use std::fs::File;
    use tempfile::tempdir;

    fn create_test_las_file(file_path: &str) {
        let builder = Builder::from((1, 4)); // LAS version 1.4
        let header = builder.into_header().unwrap();
        let mut writer = Writer::from_path(file_path, header).unwrap();

        // Create some dummy points
        for i in 0..10 {
            let mut point = Point::default();
            point.x = i as f64;
            point.y = i as f64;
            point.z = i as f64;
            writer.write_point(point).unwrap();
        }
    }

    #[test]
    fn test_process_lidar_files_success() {
        // Setup: Create a temporary directory and test files
        let dir = tempdir().unwrap();
        let input_file_path = dir.path().join("test.las");
        let output_file_path = dir.path().join("output.las");

        // Create a test .las file with some dummy data
        create_test_las_file(input_file_path.to_str().unwrap());

        // Initialize your struct with the test file paths and a simple condition
        let processor = LasProcessor {
            paths: vec![input_file_path.to_str().unwrap().to_string()],
            output_path: output_file_path.to_str().unwrap().to_string(),
            condition: Arc::new(|_point| true), // Simple condition that always returns true
            vec_size: 1000,
            strip_extra_bytes: false,
        };

        // Call the method and assert the result
        let result = processor.process_lidar_files();
        assert!(result.is_ok());

        // Additional assertions to verify the output file content can be added here
    }

    #[test]
    fn test_process_lidar_files_file_not_found() {
        // Setup: Use a non-existent file path
        let processor = LasProcessor {
            paths: vec!["non_existent_file.las".to_string()],
            output_path: "output.las".to_string(),
            condition: Arc::new(|_point| true),
            vec_size: 1000,
            strip_extra_bytes: false,
        };

        // Call the method and assert the result
        let result = processor.process_lidar_files();
        assert!(result.is_err());
    }

    #[test]
    fn test_process_lidar_files_condition_filtering() {
        // Setup: Create a temporary directory and test files
        let dir = tempdir().unwrap();
        let input_file_path = "tests/data/input1.las";
        let output_file_path = dir.path().join("output.las");

        // Create a test .las file with some dummy data

        // Initialize your struct with the test file paths and a condition that filters points
        let processor = LasProcessor {
            paths: vec![input_file_path.to_string()],
            output_path: output_file_path.to_str().unwrap().to_string(),
            condition: Arc::new(|point| point.x < 5.0), // Condition that filters points
            vec_size: 1000,
            strip_extra_bytes: false,
        };

        // Call the method and assert the result
        let result = processor.process_lidar_files();
        assert!(result.is_ok());

        // Verify that only points meeting the condition were written to the output file
        let output_file = File::open(output_file_path).unwrap();
        let mut reader = las::Reader::new(output_file).unwrap();

        for point in reader.points() {
            let point = point.unwrap();
            assert!(point.x < 5.0);
        }
    }
}
