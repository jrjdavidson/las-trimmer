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
///         "input1.laz".to_string(),
///         "input2.laz".to_string(),
///         "input3.laz".to_string(),
///     ],
///     "output.laz".to_string(),
///     |point| point.intensity > 20,
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
}
impl LasProcessor {
    /// This method creates a new `LasProcessor`. It takes as input a vector of strings representing the paths to the input LiDAR files, a string representing the path to the output LiDAR file, and a closure that takes a `las::Point` as input and returns a boolean. It returns a `LasProcessor`.
    pub fn new<F>(paths: Vec<String>, output_path: String, condition: F) -> Self
    where
        F: Fn(&Point) -> bool + Send + Sync + 'static,
    {
        Self {
            paths,
            output_path,
            vec_size: 1000 as u64, // can modulate this value to see effect on speed?
            condition: Arc::new(condition),
        }
    }

    /// This method processes the LiDAR files. It reads points from the input files, applies the condition to each point, and writes the points that meet the condition to the output file. It returns a `Result<(), MyError>`. If the method completes successfully, it returns `Ok(())`. If an error occurs, it returns `Err(MyError)`.
    pub fn process_lidar_files(&self) -> Result<(), MyError> {
        let start = Instant::now();

        // Your code here

        let vec_size = self.vec_size.clone();
        let num_threads = num_cpus::get();
        println!("Number of logical cores is {}", num_threads);

        let total_points = Arc::new(Mutex::new(0));
        let total_points_clone = Arc::clone(&total_points);
        let points_to_write_left = Arc::new(Mutex::new(0));
        let points_to_write_left_clone = Arc::clone(&points_to_write_left);

        let points_written = Arc::new(Mutex::new(0));
        let points_written_clone = Arc::clone(&points_written);
        let points_read = Arc::new(Mutex::new(0));
        let points_read_clone = Arc::clone(&points_read);

        thread::spawn(move || -> Result<(), MyError> {
            loop {
                let start = Instant::now();
                let sleep_time = 1;
                thread::sleep(Duration::from_secs(sleep_time));
                {
                    let mut points = points_written_clone
                        .lock()
                        .map_err(|_| MyError::LockError)?;
                    let mut points_r = points_read_clone.lock().map_err(|_| MyError::LockError)?;
                    let time_elapsed = start.elapsed().as_secs();

                    if *points_r == 0 && *points == 0 {
                        println!(
                            "No points were written or read in the last {} second(s).",
                            { time_elapsed }
                        );
                        continue;
                    }
                    let mut points_to_read_left =
                        total_points_clone.lock().map_err(|_| MyError::LockError)?;
                    *points_to_read_left -= *points_r;
                    let points_to_write_left = points_to_write_left_clone
                        .lock()
                        .map_err(|_| MyError::LockError)?;
                    println!(
                        "Points written/read/left in the last {} second(s): {} / {} / {} / {}",
                        time_elapsed,
                        (*points).to_formatted_string(&Locale::fr),
                        (*points_r).to_formatted_string(&Locale::fr),
                        (*points_to_read_left).to_formatted_string(&Locale::fr),
                        (*points_to_write_left).to_formatted_string(&Locale::fr),
                    );

                    *points = 0;
                    *points_r = 0;
                }
            }
        });
        let header;
        {
            let reader1 = Reader::from_path(&self.paths[0])?;
            header = reader1.header().clone()
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
            let total_points_clone = Arc::clone(&total_points);
            println!("Starting read thread {} for {:?}", i, path);

            pool.execute(move || {
                let start_time = Instant::now(); // Start the timer

                let mut reader = Reader::from_path(&path).unwrap();
                let mut points_vec = Vec::with_capacity(vec_size as usize);
                let points_remaining = reader.header().number_of_points();
                {
                    let mut total_points = total_points_clone
                        .lock()
                        .map_err(|_| MyError::LockError)
                        .unwrap();

                    *total_points += &points_remaining;
                    println!("{}/{}|| New Total:{:?}", i, total_paths, total_points);
                }
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
                            tx.send(points_vec.clone())
                                .map_err(|_| MyError::SendError)
                                .unwrap();
                            points_vec.clear(); // Clear the points_vec after sending
                        }
                    }
                }

                // Send any remaining points in the points_vec
                if !points_vec.is_empty() {
                    tx.send(points_vec).map_err(|_| MyError::SendError).unwrap();
                }

                let duration = start_time.elapsed(); // End the timer
                let points_per_second = total_points_read as f64 / duration.as_secs_f64(); // Calculate speed

                println!("Done : {:?} ({} out of {})", path, i, total_paths); // Print path number out of total                println!("Size : {:?}", reader.header().number_of_points());
                println!("Total points read: {}", total_points_read);
                println!("Time taken: {:.2?}", duration);
                println!("Read speed: {:.2} points/second", points_per_second); // Print read speed
            });
        }
        drop(tx);
        // Single writer thread
        let writer_pwc = Arc::clone(&points_written);
        let output_path = self.output_path.clone();
        let mut writer = Writer::from_path(output_path, header)?;

        while let Ok(points_vec) = rx.recv() {
            let no_of_points = points_vec.len().clone();
            {
                let mut points_tw = points_to_write_left
                    .lock()
                    .map_err(|_| MyError::LockError)
                    .unwrap();
                *points_tw += no_of_points;
            }
            for point in points_vec {
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
            *(Arc::clone(&points_to_write_left)
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
    use las::{Point, Writer};

    fn create_test_lidar_file(path: &str, points: Vec<Point>) {
        let mut writer = Writer::from_path(path, las::Header::default()).unwrap();
        for point in points {
            writer.write_point(point).unwrap();
        }
    }

    #[test]
    fn test_new() {
        let paths = vec!["input1.las".to_string(), "input2.las".to_string()];
        let output_path = "output.laz".to_string();
        let condition = |point: &Point| point.intensity > 20;

        let processor = LasProcessor::new(paths.clone(), output_path.clone(), condition);

        assert_eq!(processor.paths, paths);
        assert_eq!(processor.output_path, output_path);
        // Note: You can't directly compare closures, so we won't test the condition field here.
    }

    #[test]
    fn test_process_lidar_files() {
        let input_path = "test_input.laz";
        let output_path = "test_output.laz";
        let points = vec![
            Point {
                intensity: 10,
                ..Default::default()
            },
            Point {
                intensity: 30,
                ..Default::default()
            },
        ];

        create_test_lidar_file(input_path, points);

        let processor = LasProcessor::new(
            vec![input_path.to_string()],
            output_path.to_string(),
            |point| point.intensity > 20,
        );

        processor.process_lidar_files().unwrap();

        let mut reader = Reader::from_path(output_path).unwrap();
        let mut filtered_points = Vec::new();
        for wrapped_point in reader.points() {
            let point = wrapped_point.unwrap();
            filtered_points.push(point);
        }

        assert_eq!(filtered_points.len(), 1);
        assert_eq!(filtered_points[0].intensity, 30);
    }
}
