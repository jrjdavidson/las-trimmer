use las::Write;
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
pub mod process_points;
use crate::errors::MyError;
use crate::process_points::process_points;
use las::Point;
use las::Read;
use las::Reader;
use las::Writer;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

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
            vec_size: 10000 as u64, // can modulate this value to see effect on speed?
            condition: Arc::new(condition),
        }
    }

    /// This method processes the LiDAR files. It reads points from the input files, applies the condition to each point, and writes the points that meet the condition to the output file. It returns a `Result<(), MyError>`. If the method completes successfully, it returns `Ok(())`. If an error occurs, it returns `Err(MyError)`.
    pub fn process_lidar_files(&self) -> Result<(), MyError> {
        let vec_size = self.vec_size.clone();
        let num_threads = num_cpus::get();
        println!("Number of logical cores is {}", num_threads);

        let total_points = Arc::new(Mutex::new(0));
        let total_points_clone = Arc::clone(&total_points);

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

                    if *points_r == 0 {
                        println!("No points were written in the last {} second(s).", {
                            time_elapsed
                        });
                        *points_r = 0;
                        continue;
                    }
                    let mut total_points =
                        total_points_clone.lock().map_err(|_| MyError::LockError)?;
                    *total_points -= *points_r;
                    println!(
                        "Points written/read/left in the last {} second(s): {}/{}/{}",
                        time_elapsed, *points, *points_r, *total_points
                    );

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
            }
        });

        let reader1 = Reader::from_path(&self.paths[0])?;

        let writer = Arc::new(Mutex::new(Writer::from_path(
            &self.output_path,
            reader1.header().clone(),
        )?));

        let paths: Vec<_> = self.paths.iter().collect();
        let mut handles = vec![];
        let (tx, rx) = mpsc::channel();

        // Reader threads
        for path in paths {
            let path = path.clone();
            let tx = tx.clone();
            let condition = self.condition.clone();
            let handle = thread::spawn(move || -> Result<(), MyError> {
                let mut reader = Reader::from_path(path)?;
                let mut points_vec = Vec::with_capacity(vec_size as usize);
                loop {
                    let points_read = reader.read_n_into(vec_size, &mut points_vec)?;
                    if points_read == 0 {
                        break;
                    }

                    let filtered_points: Vec<Point> = points_vec
                        .drain(..)
                        .filter(|point| condition(point))
                        .collect();

                    if !filtered_points.is_empty() {
                        tx.send(filtered_points).map_err(|_| MyError::SendError)?;
                    }
                }
                Ok(())
            });
            handles.push(handle);
        }

        // Single writer thread
        let writer_handle = thread::spawn(move || -> Result<(), MyError> {
            while let Ok(points_vec) = rx.recv() {
                let mut writer = writer.lock().map_err(|_| MyError::LockError)?;
                for point in points_vec {
                    writer.write(point)?;
                }
            }
            Ok(())
        });

        writer_handle.join().map_err(|_| MyError::ThreadError)??;

        Ok(())
    }
}
