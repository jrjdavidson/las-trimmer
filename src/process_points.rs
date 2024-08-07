use crate::errors::MyError;
use crate::thread;
use las::Point;
use las::Read;
use las::Reader;
use las::Write;
use las::Writer;
use std::cmp;
use std::sync::Arc;
use std::sync::Mutex;
/// `process_points` is a function that reads points from a LiDAR file, applies a condition to each point, and writes the points that meet the condition to an output file.
///
/// # Arguments
///
/// * `reader`: A mutable reference to a `las::Reader` object. This object is used to read points from the input LiDAR file.
/// * `writer`: A mutable reference to an `Arc<Mutex<Writer<W>>>` object. This object is used to write points to the output LiDAR file.
/// * `vec`: A mutable reference to a `Vec<Point>`. This vector is used to temporarily store points read from the input file.
/// * `points_read`: A reference to an `Arc<Mutex<u64>>`. This object is used to keep track of the total number of points read from the input file.
/// * `points_written`: A mutable reference to a `Mutex<i32>`. This object is used to keep track of the total number of points written to the output file.
/// * `points_per_cycle`: The maximum number of points to be read from the input file in one cycle of the loop.
/// * `vec_size`: The maximum number of points that can be stored in `vec`.
/// * `condition`: A closure that takes a `Point` as input and returns a boolean. This closure is applied to each point read from the input file. Only points for which the closure returns `true` are written to the output file.
///
/// # Returns
///
/// * `Result<(), MyError>`: If the function completes successfully, it returns `Ok(())`. If an error occurs, it returns `Err(MyError)`.
///
/// # Errors
///
/// This function will return an error if:
/// * It fails to read points from the input file.
/// * It fails to write points to the output file.
/// * It fails to acquire a lock on `points_read` or `points_written`.
///
/// # Example
///
/// ```rust
/// let result = process_points(
///     &mut reader,
///     &mut Arc::clone(&writer),
///     &mut points_vec,
///     &points_read,
///     &mut points_written,
///     points_per_cycle,
///     vec_size,
///     |_| true,
/// );
/// assert!(result.is_ok());
/// ```
pub fn process_points<W: std::io::Write + std::io::Seek + std::fmt::Debug + std::marker::Send>(
    reader: &mut Reader,
    writer: &mut Arc<Mutex<Writer<W>>>,
    vec: &mut Vec<Point>,
    points_read: &Arc<Mutex<u64>>,
    points_written: &Mutex<i32>,
    points_per_cycle: u64,
    vec_size: u64,
    condition: impl Fn(&Point) -> bool,
) -> Result<(), MyError> {
    let mut points_remaining = points_per_cycle.clone();
    loop {
        let to_be_read = cmp::min(points_remaining, vec_size);

        let points_read_from_reader = reader.read_n_into(to_be_read, vec)?;
        if points_read_from_reader == 0 {
            println!("Thread Finished:{:?}", thread::current().name());

            break;
        }
        points_remaining -= points_read_from_reader;
        {
            let mut points = points_read.lock().map_err(|_| MyError::LockError)?;
            *points += points_read_from_reader as u64;
        }
        while let Some(point) = vec.pop() {
            if condition(&point) {
                {
                    writer
                        .lock()
                        .map_err(|_| MyError::LockError)?
                        .write(point)?;
                }
                {
                    let mut points_w = points_written.lock().map_err(|_| MyError::LockError)?;
                    *points_w += 1;
                }
            }
        }
    }
    Ok(())
}
