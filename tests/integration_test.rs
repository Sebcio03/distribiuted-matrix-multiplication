use distribiuted_matrix_multiplication::matrix::Matrix;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_matrix_file_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_matrix.txt");

    let original = Matrix::from_vec(
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        3,
        3,
    )
    .unwrap();

    original.save_to_file(&file_path).unwrap();

    let loaded = Matrix::load_from_file(&file_path).unwrap();

    assert_eq!(original.rows, loaded.rows);
    assert_eq!(original.cols, loaded.cols);
    assert_eq!(original.data, loaded.data);
}

#[test]
fn test_matrix_multiplication_correctness() {
    // Test with known result
    // [1 2]   [5 6]   [19 22]
    // [3 4] * [7 8] = [43 50]
    let a = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::from_vec(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    let c = a.multiply(&b).unwrap();

    assert_eq!(c.get(0, 0).unwrap(), 19.0);
    assert_eq!(c.get(0, 1).unwrap(), 22.0);
    assert_eq!(c.get(1, 0).unwrap(), 43.0);
    assert_eq!(c.get(1, 1).unwrap(), 50.0);
}

#[test]
fn test_matrix_multiplication_identity() {
    let size = 5;
    let mut identity = Matrix::new(size, size);
    for i in 0..size {
        identity.set(i, i, 1.0).unwrap();
    }

    let data: Vec<f64> = (0..size * size).map(|x| x as f64).collect();
    let test_matrix = Matrix::from_vec(data, size, size).unwrap();

    let result = test_matrix.multiply(&identity).unwrap();
    assert_eq!(result.data, test_matrix.data);
}

#[test]
fn test_chunk_operations() {
    // Create a 6x4 matrix
    let data: Vec<f64> = (1..=24).map(|x| x as f64).collect();
    let matrix = Matrix::from_vec(data, 6, 4).unwrap();

    // Get row chunk (rows 2-4)
    let row_chunk = matrix.get_row_chunk(2, 2).unwrap();
    assert_eq!(row_chunk.rows, 2);
    assert_eq!(row_chunk.cols, 4);
    assert_eq!(row_chunk.get(0, 0).unwrap(), 9.0); // row 2, col 0

    // Get column chunk (cols 1-3)
    let col_chunk = matrix.get_col_chunk(1, 2).unwrap();
    assert_eq!(col_chunk.rows, 6);
    assert_eq!(col_chunk.cols, 2);
    assert_eq!(col_chunk.get(0, 0).unwrap(), 2.0); // row 0, col 1
}

#[test]
fn test_distributed_multiplication_simulation() {
    // Simulate distributed multiplication without MPI
    // Matrix A: 4x3, Matrix B: 3x2
    let a_data: Vec<f64> = (1..=12).map(|x| x as f64).collect();
    let b_data: Vec<f64> = (1..=6).map(|x| x as f64).collect();
    
    let matrix_a = Matrix::from_vec(a_data, 4, 3).unwrap();
    let matrix_b = Matrix::from_vec(b_data, 3, 2).unwrap();

    // Simulate 2 workers:
    // Worker 1: rows 0-2 of A, cols 0-1 of B
    // Worker 2: rows 2-4 of A, cols 0-1 of B
    
    let row_chunk_1 = matrix_a.get_row_chunk(0, 2).unwrap();
    let col_chunk_1 = matrix_b.get_col_chunk(0, 2).unwrap();
    let result_1 = Matrix::multiply_chunks(&row_chunk_1, &col_chunk_1).unwrap();

    let row_chunk_2 = matrix_a.get_row_chunk(2, 2).unwrap();
    let col_chunk_2 = matrix_b.get_col_chunk(0, 2).unwrap();
    let result_2 = Matrix::multiply_chunks(&row_chunk_2, &col_chunk_2).unwrap();

    // Compute expected result
    let expected = matrix_a.multiply(&matrix_b).unwrap();

    // Verify worker 1's result matches expected rows 0-1
    for i in 0..result_1.rows {
        for j in 0..result_1.cols {
            assert_eq!(
                result_1.get(i, j).unwrap(),
                expected.get(i, j).unwrap(),
                "Worker 1 result mismatch at ({}, {})",
                i,
                j
            );
        }
    }

    // Verify worker 2's result matches expected rows 2-3
    for i in 0..result_2.rows {
        for j in 0..result_2.cols {
            assert_eq!(
                result_2.get(i, j).unwrap(),
                expected.get(i + 2, j).unwrap(),
                "Worker 2 result mismatch at ({}, {})",
                i,
                j
            );
        }
    }
}

#[test]
fn test_matrix_file_format_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("format_test.txt");

    // Test various formats
    let content = "1.5  2.5   3.5\n4.0 5.0 6.0\n7.0\t8.0\t9.0\n";
    fs::write(&file_path, content).unwrap();

    let matrix = Matrix::load_from_file(&file_path).unwrap();
    assert_eq!(matrix.rows, 3);
    assert_eq!(matrix.cols, 3);
    assert!((matrix.get(0, 0).unwrap() - 1.5).abs() < 0.001);
    assert!((matrix.get(2, 2).unwrap() - 9.0).abs() < 0.001);
}

#[test]
fn test_large_matrix_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_matrix.txt");

    // Create a 100x100 matrix
    let size = 100;
    let mut matrix = Matrix::new(size, size);
    for i in 0..size {
        for j in 0..size {
            matrix.set(i, j, (i * size + j) as f64).unwrap();
        }
    }

    // Save and load
    matrix.save_to_file(&file_path).unwrap();
    let loaded = Matrix::load_from_file(&file_path).unwrap();

    assert_eq!(matrix.rows, loaded.rows);
    assert_eq!(matrix.cols, loaded.cols);
    assert_eq!(matrix.data, loaded.data);
}

#[test]
fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Test non-existent file
    let non_existent = temp_dir.path().join("nonexistent.txt");
    assert!(Matrix::load_from_file(&non_existent).is_err());

    // Test invalid format
    let invalid_file = temp_dir.path().join("invalid.txt");
    fs::write(&invalid_file, "not a number 2.0\n").unwrap();
    assert!(Matrix::load_from_file(&invalid_file).is_err());

    // Test incompatible multiplication
    let a = Matrix::new(2, 3);
    let b = Matrix::new(4, 2);
    assert!(a.multiply(&b).is_err());
}
