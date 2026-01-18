// Unit tests for Matrix module

use distribiuted_matrix_multiplication::matrix::Matrix;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_matrix_creation() {
    let m = Matrix::new(3, 4);
    assert_eq!(m.rows, 3);
    assert_eq!(m.cols, 4);
    assert_eq!(m.data.len(), 12);
}

#[test]
fn test_matrix_from_vec() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let m = Matrix::from_vec(data.clone(), 2, 3).unwrap();
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 3);
    assert_eq!(m.data, data);
}

#[test]
fn test_load_and_save() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "1.0 2.0 3.0").unwrap();
    writeln!(file, "4.0 5.0 6.0").unwrap();
    file.flush().unwrap();

    let m = Matrix::load_from_file(file.path()).unwrap();
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 3);
    assert_eq!(m.get(0, 0).unwrap(), 1.0);
    assert_eq!(m.get(1, 2).unwrap(), 6.0);

    let output_file = NamedTempFile::new().unwrap();
    m.save_to_file(output_file.path()).unwrap();

    let m2 = Matrix::load_from_file(output_file.path()).unwrap();
    assert_eq!(m.data, m2.data);
}

#[test]
fn test_multiply() {
    let a = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::from_vec(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    let c = a.multiply(&b).unwrap();

    // [1 2]   [5 6]   [19 22]
    // [3 4] * [7 8] = [43 50]
    assert_eq!(c.get(0, 0).unwrap(), 19.0);
    assert_eq!(c.get(0, 1).unwrap(), 22.0);
    assert_eq!(c.get(1, 0).unwrap(), 43.0);
    assert_eq!(c.get(1, 1).unwrap(), 50.0);
}

#[test]
fn test_multiply_incompatible_dimensions() {
    let a = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::from_vec(vec![1.0, 2.0, 3.0], 3, 1).unwrap();
    assert!(a.multiply(&b).is_err());
}

#[test]
fn test_get_set() {
    let mut m = Matrix::new(3, 3);
    m.set(1, 2, 42.0).unwrap();
    assert_eq!(m.get(1, 2).unwrap(), 42.0);
    assert_eq!(m.get(0, 0).unwrap(), 0.0);
}

#[test]
fn test_get_set_out_of_bounds() {
    let mut m = Matrix::new(3, 3);
    assert!(m.get(3, 0).is_err());
    assert!(m.get(0, 3).is_err());
    assert!(m.set(3, 0, 1.0).is_err());
    assert!(m.set(0, 3, 1.0).is_err());
}

#[test]
fn test_get_row() {
    let m = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    let row = m.get_row(0).unwrap();
    assert_eq!(row, &[1.0, 2.0, 3.0]);
    let row = m.get_row(1).unwrap();
    assert_eq!(row, &[4.0, 5.0, 6.0]);
}

#[test]
fn test_get_col() {
    let m = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    let col = m.get_col(0).unwrap();
    assert_eq!(col, vec![1.0, 4.0]);
    let col = m.get_col(1).unwrap();
    assert_eq!(col, vec![2.0, 5.0]);
    let col = m.get_col(2).unwrap();
    assert_eq!(col, vec![3.0, 6.0]);
}

#[test]
fn test_get_row_chunk() {
    let data: Vec<f64> = (1..=12).map(|x| x as f64).collect();
    let m = Matrix::from_vec(data, 4, 3).unwrap();
    
    let chunk = m.get_row_chunk(1, 2).unwrap();
    assert_eq!(chunk.rows, 2);
    assert_eq!(chunk.cols, 3);
    assert_eq!(chunk.get(0, 0).unwrap(), 4.0); // row 1, col 0
    assert_eq!(chunk.get(1, 2).unwrap(), 9.0); // row 2, col 2
}

#[test]
fn test_get_col_chunk() {
    let data: Vec<f64> = (1..=12).map(|x| x as f64).collect();
    let m = Matrix::from_vec(data, 4, 3).unwrap();
    
    let chunk = m.get_col_chunk(1, 2).unwrap();
    assert_eq!(chunk.rows, 4);
    assert_eq!(chunk.cols, 2);
    assert_eq!(chunk.get(0, 0).unwrap(), 2.0); // row 0, col 1
    assert_eq!(chunk.get(0, 1).unwrap(), 3.0); // row 0, col 2
    assert_eq!(chunk.get(3, 1).unwrap(), 12.0); // row 3, col 2
}

#[test]
fn test_multiply_chunks() {
    // Test chunk multiplication
    let row_chunk = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let col_chunk = Matrix::from_vec(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    let result = Matrix::multiply_chunks(&row_chunk, &col_chunk).unwrap();
    
    assert_eq!(result.rows, 2);
    assert_eq!(result.cols, 2);
    assert_eq!(result.get(0, 0).unwrap(), 19.0);
    assert_eq!(result.get(0, 1).unwrap(), 22.0);
}

#[test]
fn test_load_empty_file() {
    let file = NamedTempFile::new().unwrap();
    assert!(Matrix::load_from_file(file.path()).is_err());
}

#[test]
fn test_load_inconsistent_columns() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "1.0 2.0 3.0").unwrap();
    writeln!(file, "4.0 5.0").unwrap(); // Different number of columns
    file.flush().unwrap();
    
    assert!(Matrix::load_from_file(file.path()).is_err());
}

#[test]
fn test_load_with_empty_lines() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "1.0 2.0").unwrap();
    writeln!(file, "").unwrap(); // Empty line
    writeln!(file, "3.0 4.0").unwrap();
    file.flush().unwrap();
    
    let m = Matrix::load_from_file(file.path()).unwrap();
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
}

#[test]
fn test_from_vec_invalid_size() {
    let data = vec![1.0, 2.0, 3.0];
    assert!(Matrix::from_vec(data, 2, 2).is_err());
}

#[test]
fn test_large_matrix_operations() {
    // Test with a larger matrix to ensure performance
    let size = 100;
    let data: Vec<f64> = (0..size * size).map(|x| x as f64).collect();
    let a = Matrix::from_vec(data, size, size).unwrap();
    let b = Matrix::from_vec(vec![1.0; size * size], size, size).unwrap();
    
    let result = a.multiply(&b).unwrap();
    assert_eq!(result.rows, size);
    assert_eq!(result.cols, size);
    // First row should be sum of first row of a
    let first_row_sum: f64 = (0..size).map(|i| i as f64).sum();
    assert!((result.get(0, 0).unwrap() - first_row_sum).abs() < 0.001);
}

