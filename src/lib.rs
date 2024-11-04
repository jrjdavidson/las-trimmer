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
///     vec!["output.laz".to_string()],
///     vec![Arc::new(|point| point.intensity > 20)],
///     false
/// );
///
/// processor.process_lidar_files().unwrap();
/// ```
pub mod errors;
use crate::errors::MyError;
use crossbeam::channel;
use las::Point;
use las::Reader;
use las::Writer;
use num_format::{Locale, ToFormattedString};
use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use threadpool::ThreadPool;

pub type SharedFunction = Arc<dyn Fn(&Point) -> bool + Send + Sync>;
/// `LasProcessor` is a struct that represents a processor for LiDAR files.
pub struct LasProcessor {
    /// A vector of strings representing the paths to the input LiDAR files.
    paths: Vec<String>,
    /// A vector of strings representing the paths to the output LiDAR files.
    output_paths: Vec<String>,
    /// A vector of `Arc` containing closures that take a `Point` as input and return a boolean.
    /// Each closure is applied to each point read from the input files. Only points for which the closure returns `true` are written to the corresponding output file.
    conditions: Vec<SharedFunction>,
    vec_size: u64,
    strip_extra_bytes: bool,
}

impl LasProcessor {
    /// This method creates a new `LasProcessor`. It takes as input a vector of strings representing the paths to the input LiDAR files,
    /// a vector of strings representing the paths to the output LiDAR files, and a vector of closures that take a `las::Point` as input and return a boolean.
    /// It returns a `LasProcessor`.
    pub fn new(
        paths: Vec<String>,
        output_paths: Vec<String>,
        conditions: Vec<SharedFunction>,
        strip_extra_bytes: bool,
    ) -> Self
where {
        Self {
            paths,
            output_paths,
            vec_size: 100000, // can modulate this value to see effect on speed
            conditions,
            strip_extra_bytes,
        }
    }

    /// This method processes the LiDAR files. It reads points from the input files, applies the condition to each point, and writes the points that meet the condition to the output file. It returns a `Result<(), MyError>`. If the method completes successfully, it returns `Ok(())`. If an error occurs, it returns `Err(MyError)`.
    pub fn process_lidar_files(&self) -> Result<(), MyError> {
        let start = Instant::now();
        let number_locale = &Locale::en;

        let vec_size = self.vec_size;
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
                std::thread::sleep(Duration::from_secs(sleep_time));
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
            let reader1 = Reader::from_path(&self.paths[0]).unwrap();
            let old_header = reader1.header().clone();
            if self.strip_extra_bytes {
                let format_u8 = old_header.point_format().to_u8().unwrap();
                println!("Old header format : {}", format_u8);

                let mut new_format = Format::new(format_u8).unwrap();
                let mut builder = Builder::new(old_header.into_raw().unwrap()).unwrap();
                new_format.extra_bytes = 0;
                builder.point_format = new_format;

                header = builder.into_header().unwrap();
            } else {
                header = old_header;
            }
        }

        let (tx, rx) = channel::bounded(20);
        let sendthreads = num_threads - 2 - self.output_paths.len();
        let pool = ThreadPool::new(sendthreads);

        // Reader threads
        let total_paths = self.paths.len();
        // could use rayon for iter?
        for (i, path) in self.paths.iter().enumerate() {
            let path = path.clone();
            let tx = tx.clone();
            let conditions = self.conditions.clone();
            let points_read_clone = Arc::clone(&points_read);
            let total_points_to_read_clone = Arc::clone(&total_points_to_read);
            let total_points_to_write_clone = Arc::clone(&total_points_to_write);

            println!("Starting read thread {} for {:?}", i, path);
            pool.execute(move || {
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

                let start_time = Instant::now();

                let mut reader = Reader::from_path(&path).unwrap();
                let mut points_vecs: Vec<Vec<Point>> =
                    vec![Vec::with_capacity(vec_size as usize); conditions.len()];
                let mut total_points_read = 0;

                for wrapped_point in reader.points() {
                    let point = wrapped_point.unwrap();
                    total_points_read += 1;

                    {
                        let mut points = points_read_clone
                            .lock()
                            .map_err(|_| MyError::LockError)
                            .unwrap();
                        *points += 1;
                    }

                    for (j, condition) in conditions.iter().enumerate() {
                        if condition(&point) {
                            points_vecs[j].push(point.clone());
                            if points_vecs[j].len() >= vec_size.try_into().unwrap() {
                                {
                                    let mut points_tw = total_points_to_write_clone
                                        .lock()
                                        .map_err(|_| MyError::LockError)
                                        .unwrap();
                                    *points_tw += points_vecs[j].len();
                                }
                                tx.send((j, points_vecs[j].clone()))
                                    .map_err(|_| MyError::SendError)
                                    .unwrap();
                                points_vecs[j].clear();
                            }
                        }
                    }
                }

                for (j, points_vec) in points_vecs.into_iter().enumerate() {
                    if !points_vec.is_empty() {
                        {
                            let mut points_tw = total_points_to_write_clone
                                .lock()
                                .map_err(|_| MyError::LockError)
                                .unwrap();
                            *points_tw += points_vec.len();
                        }
                        tx.send((j, points_vec))
                            .map_err(|_| MyError::SendError)
                            .unwrap();
                    }
                }

                let duration = start_time.elapsed();
                let points_per_second = total_points_read as f64 / duration.as_secs_f64();

                println!("Done : {:?} ({} out of {})", path, i, total_paths);
                println!(
                    "Size : {:?}",
                    reader
                        .header()
                        .number_of_points()
                        .to_formatted_string(number_locale)
                );
                println!(
                    "Total points read: {}",
                    total_points_read.to_formatted_string(number_locale)
                );
                println!("Time taken: {:.2?}", duration);
                println!("Read speed: {:.2} points/second", points_per_second);
            });
        }

        drop(tx);

        // Writer threads
        let mut writers: Vec<Writer<BufWriter<File>>> = Vec::new();
        for output_path in &self.output_paths {
            let writer = Writer::from_path(output_path, header.clone()).unwrap();
            writers.push(writer);
        }
        while let Ok((index, points_vec)) = rx.recv() {
            let no_of_points = points_vec.len();

            for mut point in points_vec {
                if self.strip_extra_bytes {
                    point.extra_bytes.clear();
                }
                writers[index].write_point(point).unwrap();
            }
            {
                let mut points_w = points_written
                    .lock()
                    .map_err(|_| MyError::LockError)
                    .unwrap();
                *points_w += no_of_points;
            }
        }

        let points_w = points_written
            .lock()
            .map_err(|_| MyError::LockError)
            .unwrap();
        let points_r = points_read.lock().map_err(|_| MyError::LockError).unwrap();

        println!(
            "Total points read/written: {}/{}",
            (*points_r).to_formatted_string(number_locale),
            (*points_w).to_formatted_string(number_locale)
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
            let point = las::Point {
                x: i as f64,
                y: i as f64,
                z: i as f64,
                ..Default::default()
            };
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
            output_paths: vec![output_file_path.to_str().unwrap().to_string()],
            conditions: vec![Arc::new(|_point| true)], // Simple condition that always returns true
            vec_size: 100000,
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
            output_paths: vec!["output.las".to_string()],
            conditions: vec![Arc::new(|_point| true)],
            vec_size: 100000,
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
            output_paths: vec![output_file_path.to_str().unwrap().to_string()],
            conditions: vec![Arc::new(|point| point.x < 5.0)], // Condition that filters points
            vec_size: 100000,
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

    #[test]
    fn test_process_lidar_files_multiple_conditions() {
        // Setup: Create a temporary directory and test files
        let dir = tempdir().unwrap();
        let input_file_path = dir.path().join("test.las");
        let output_file_path1 = dir.path().join("output1.las");
        let output_file_path2 = dir.path().join("output2.las");

        // Create a test .las file with some dummy data
        create_test_las_file(input_file_path.to_str().unwrap());

        // Initialize your struct with the test file paths and multiple conditions
        let processor = LasProcessor {
            paths: vec![input_file_path.to_str().unwrap().to_string()],
            output_paths: vec![
                output_file_path1.to_str().unwrap().to_string(),
                output_file_path2.to_str().unwrap().to_string(),
            ],
            conditions: vec![
                Arc::new(|point: &Point| point.x < 5.0), // Condition for output1
                Arc::new(|point: &Point| point.x >= 5.0), // Condition for output2
            ],
            vec_size: 100000,
            strip_extra_bytes: false,
        };

        // Call the method and assert the result
        let result = processor.process_lidar_files();
        assert!(result.is_ok());

        // Verify that points meeting the first condition were written to the first output file
        let output_file1 = File::open(output_file_path1).unwrap();
        let mut reader1 = las::Reader::new(output_file1).unwrap();
        for point in reader1.points() {
            let point = point.unwrap();
            assert!(point.x < 5.0);
        }

        // Verify that points meeting the second condition were written to the second output file
        let output_file2 = File::open(output_file_path2).unwrap();
        let mut reader2 = las::Reader::new(output_file2).unwrap();
        for point in reader2.points() {
            let point = point.unwrap();
            assert!(point.x >= 5.0);
        }
    }

    #[test]
    fn test_process_lidar_files_empty_input() {
        // Setup: Create a temporary directory and test files
        let dir = tempdir().unwrap();
        let input_file_path = dir.path().join("empty.las");
        let output_file_path = dir.path().join("output.las");
        // Create an empty test .las file
        let builder = Builder::from((1, 4)); // LAS version 1.4
        let header = builder.into_header().unwrap();
        println!("{}", input_file_path.to_str().unwrap());
        {
            let _writer = Writer::from_path(input_file_path.to_str().unwrap(), header).unwrap();
        }

        // Initialize your struct with the test file paths and a simple condition
        let processor = LasProcessor {
            paths: vec![input_file_path.to_str().unwrap().to_string()],
            output_paths: vec![output_file_path.to_str().unwrap().to_string()],
            conditions: vec![Arc::new(|_point| true)], // Simple condition that always returns true
            vec_size: 100000,
            strip_extra_bytes: false,
        };

        // Call the method and assert the result
        let result = processor.process_lidar_files();
        assert!(result.is_ok());

        // Verify that the output file is also empty
        let output_file = File::open(output_file_path).unwrap();
        let mut reader = las::Reader::new(output_file).unwrap();
        assert!(reader.points().next().is_none());
    }

    #[test]
    fn test_process_lidar_files_strip_extra_bytes() {
        // Setup: Create a temporary directory and test files
        let dir = tempdir().unwrap();
        let input_file_path = dir.path().join("test.las");
        let output_file_path = dir.path().join("output.las");

        // Create a test .las file with some dummy data
        create_test_las_file(input_file_path.to_str().unwrap());

        // Initialize your struct with the test file paths and a simple condition
        let processor = LasProcessor {
            paths: vec![input_file_path.to_str().unwrap().to_string()],
            output_paths: vec![output_file_path.to_str().unwrap().to_string()],
            conditions: vec![Arc::new(|_point| true)], // Simple condition that always returns true
            vec_size: 100000,
            strip_extra_bytes: true, // Enable strip_extra_bytes
        };

        // Call the method and assert the result
        let result = processor.process_lidar_files();
        assert!(result.is_ok());

        // Verify that the output file has points with empty extra_bytes
        let output_file = File::open(output_file_path).unwrap();
        let mut reader = las::Reader::new(output_file).unwrap();
        for point in reader.points() {
            let point = point.unwrap();
            assert!(point.extra_bytes.is_empty());
        }
    }
}
