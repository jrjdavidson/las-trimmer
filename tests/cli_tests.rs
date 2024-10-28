use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_always_true() {
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("test.las");
    let output_file_path = dir.path().join("output.las");

    // Create a test .las file with some dummy data
    create_test_las_file(input_file_path.to_str().unwrap());

    let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
    cmd.arg("--input")
        .arg(input_file_path)
        .arg("--output")
        .arg(output_file_path.clone())
        .arg("--filter")
        .arg("always-true");

    cmd.assert().success();

    // Verify that the output file exists and contains the expected data
    assert!(output_file_path.exists());
    let output_file = fs::File::open(output_file_path).unwrap();
    let mut reader = las::Reader::new(output_file).unwrap();
    let points: Vec<_> = reader.points().collect();
    assert_eq!(points.len(), 10); // Assuming the test file has 10 points
}

#[test]
fn test_cli_always_false() {
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("test.las");
    let output_file_path = dir.path().join("output.las");

    // Create a test .las file with some dummy data
    create_test_las_file(input_file_path.to_str().unwrap());

    let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
    cmd.arg("--input")
        .arg(input_file_path)
        .arg("--output")
        .arg(output_file_path.clone())
        .arg("--filter")
        .arg("always-false");

    cmd.assert().success();

    // Verify that the output file exists and is empty
    assert!(output_file_path.exists());
    let output_file = fs::File::open(output_file_path).unwrap();
    let mut reader = las::Reader::new(output_file).unwrap();
    assert!(reader.points().next().is_none());
}

// fn test_cli_crop() {
//     let dir = tempdir().unwrap();
//     let input_file_path = dir.path().join("test.las");
//     let output_file_path = dir.path().join("output.las");

//     // Create a test .las file with some dummy data
//     create_test_las_file(input_file_path.to_str().unwrap());

//     let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
//     cmd.arg("--input")
//         .arg(input_file_path)
//         .arg("--output")
//         .arg(output_file_path.clone())
//         .arg("crop")
//         .arg("--min-x")
//         .arg("0.0")
//         .arg("--max-x")
//         .arg("5.0");

//     cmd.assert().success();

//     // Verify that the output file exists and contains the expected data
//     assert!(output_file_path.exists());
//     let output_file = fs::File::open(output_file_path).unwrap();
//     let mut reader = las::Reader::new(output_file).unwrap();
//     for point in reader.points() {
//         let point = point.unwrap();
//         assert!(point.x >= 0.0 && point.x < 5.0);
//     }
// }

#[test]
fn test_cli_real_data_always_true() {
    let dir = tempdir().unwrap();
    let output_file_path = dir.path().join("output.laz");

    let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
    cmd.arg("--input")
        .arg("tests/data/input1.las")
        .arg("--output")
        .arg(output_file_path.clone())
        .arg("--filter")
        .arg("always-true");

    cmd.assert().success();

    // Verify that the output file exists and contains the expected data
    assert!(output_file_path.exists());
    let output_file = fs::File::open(output_file_path).unwrap();
    let mut reader = las::Reader::new(output_file).unwrap();
    let points: Vec<_> = reader.points().collect();
    assert!(!points.is_empty()); // Ensure the output file is not empty
}

#[test]
fn test_cli_real_data_multiple_files() {
    let dir = tempdir().unwrap();
    let output_file_path = dir.path().join("output.laz");

    let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
    cmd.arg("--input")
        .arg("tests/data/input1.las")
        .arg("--input")
        .arg("tests/data/input2.las")
        .arg("--output")
        .arg(output_file_path.clone())
        .arg("--filter")
        .arg("always-true");

    cmd.assert().success();

    // Verify that the output file exists and contains the expected data
    assert!(output_file_path.exists());
    let output_file = fs::File::open(output_file_path).unwrap();
    let mut reader = las::Reader::new(output_file).unwrap();
    let points: Vec<_> = reader.points().collect();
    assert!(!points.is_empty()); // Ensure the output file is not empty
    println!("{}", points.len());
    assert!(points.len() == 950253); // Ensure it contains points from both input files
}

#[test]
fn test_cli_multiple_output_files() {
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("test.las");
    let output_file_path1 = dir.path().join("output1.las");
    let output_file_path2 = dir.path().join("output2.las");

    // Create a test .las file with some dummy data
    create_test_las_file(input_file_path.to_str().unwrap());

    let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
    cmd.arg("--input")
        .arg(input_file_path)
        .arg("--output")
        .arg(output_file_path1.clone())
        .arg("--filter")
        .arg("always-true")
        .arg("--output")
        .arg(output_file_path2.clone())
        .arg("--filter")
        .arg("always-false");

    cmd.assert().success();

    // Verify that the first output file exists and contains the expected data
    assert!(output_file_path1.exists());
    let output_file1 = fs::File::open(output_file_path1).unwrap();
    let reader1 = las::Reader::new(output_file1).unwrap();
    assert_eq!(reader1.header().number_of_points(), 10); // Assuming the test file has 10 points

    // Verify that the second output file exists and contains the expected data
    assert!(output_file_path2.exists());
    let output_file2 = fs::File::open(output_file_path2).unwrap();
    let reader2 = las::Reader::new(output_file2).unwrap();
    assert_eq!(reader2.header().number_of_points(), 0); // Assuming the test file has 10 points
}

#[test]
fn test_cli_mismatched_filters_and_outputs() {
    let dir = tempdir().unwrap();
    let input_file_path = dir.path().join("test.las");
    let output_file_path1 = dir.path().join("output1.las");
    let output_file_path2 = dir.path().join("output2.las");

    // Create a test .las file with some dummy data
    create_test_las_file(input_file_path.to_str().unwrap());

    let mut cmd = Command::cargo_bin("las_trimmer").unwrap();
    cmd.arg("--input")
        .arg(input_file_path)
        .arg("--output")
        .arg(output_file_path1.clone())
        .arg("--output")
        .arg(output_file_path2.clone())
        .arg("--filter")
        .arg("always-true");

    cmd.assert().failure().stderr(predicates::str::contains(
        "Output paths number must match the number of filter arguments",
    ));
}

fn create_test_las_file(file_path: &str) {
    let builder = las::Builder::from((1, 4)); // LAS version 1.4
    let header = builder.into_header().unwrap();
    let mut writer = las::Writer::from_path(file_path, header).unwrap();

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
