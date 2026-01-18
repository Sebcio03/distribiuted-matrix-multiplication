use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Matrix {
    pub data: Vec<f64>,
    pub rows: usize,
    pub cols: usize,
}

impl Matrix {
    /// Create a new matrix with the given dimensions
    pub fn new(rows: usize, cols: usize) -> Self {
        Matrix {
            data: vec![0.0; rows * cols],
            rows,
            cols,
        }
    }

    /// Create a matrix from a vector of data
    pub fn from_vec(data: Vec<f64>, rows: usize, cols: usize) -> Result<Self, String> {
        if data.len() != rows * cols {
            return Err(format!(
                "Data length {} does not match dimensions {}x{}",
                data.len(),
                rows,
                cols
            ));
        }
        Ok(Matrix { data, rows, cols })
    }

    /// Load a matrix from a text file
    /// Format: space-separated values, one row per line
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut rows = Vec::new();
        let mut num_cols = None;

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Failed to read line {}: {}", line_num + 1, e))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            let values: Result<Vec<f64>, _> = trimmed
                .split_whitespace()
                .map(|s| s.parse::<f64>())
                .collect();

            let values = values.map_err(|e| {
                format!(
                    "Failed to parse value on line {}: {}",
                    line_num + 1, e
                )
            })?;

            if values.is_empty() {
                continue; // Skip lines with no values
            }

            // Check that all rows have the same number of columns
            match num_cols {
                Some(n) if n != values.len() => {
                    return Err(format!(
                        "Inconsistent column count: expected {}, found {} on line {}",
                        n,
                        values.len(),
                        line_num + 1
                    ));
                }
                None => num_cols = Some(values.len()),
                _ => {}
            }

            rows.push(values);
        }

        if rows.is_empty() {
            return Err("Matrix file is empty".to_string());
        }

        let cols = num_cols.ok_or("No valid rows found")?;
        let rows_count = rows.len();
        let data: Vec<f64> = rows.into_iter().flatten().collect();

        Ok(Matrix {
            data,
            rows: rows_count,
            cols,
        })
    }

    /// Save a matrix to a text file
    /// Format: space-separated values, one row per line
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        let mut writer = BufWriter::new(file);

        for i in 0..self.rows {
            let row_start = i * self.cols;
            let row_end = row_start + self.cols;
            let row = &self.data[row_start..row_end];

            // Write row values separated by spaces
            for (j, &value) in row.iter().enumerate() {
                if j > 0 {
                    write!(writer, " ").map_err(|e| format!("Failed to write: {}", e))?;
                }
                write!(writer, "{}", value).map_err(|e| format!("Failed to write: {}", e))?;
            }
            writeln!(writer).map_err(|e| format!("Failed to write newline: {}", e))?;
        }

        writer
            .flush()
            .map_err(|e| format!("Failed to flush file: {}", e))?;

        Ok(())
    }

    /// Get a value at a specific position
    pub fn get(&self, row: usize, col: usize) -> Result<f64, String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Index out of bounds: ({}, {}) for matrix {}x{}",
                row, col, self.rows, self.cols
            ));
        }
        Ok(self.data[row * self.cols + col])
    }

    /// Set a value at a specific position
    pub fn set(&mut self, row: usize, col: usize, value: f64) -> Result<(), String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Index out of bounds: ({}, {}) for matrix {}x{}",
                row, col, self.rows, self.cols
            ));
        }
        self.data[row * self.cols + col] = value;
        Ok(())
    }

    /// Get a row as a slice
    pub fn get_row(&self, row: usize) -> Result<&[f64], String> {
        if row >= self.rows {
            return Err(format!("Row index {} out of bounds for {} rows", row, self.rows));
        }
        let start = row * self.cols;
        let end = start + self.cols;
        Ok(&self.data[start..end])
    }

    /// Get a column as a vector
    pub fn get_col(&self, col: usize) -> Result<Vec<f64>, String> {
        if col >= self.cols {
            return Err(format!("Column index {} out of bounds for {} cols", col, self.cols));
        }
        Ok((0..self.rows)
            .map(|row| self.data[row * self.cols + col])
            .collect())
    }

    /// Get a submatrix (row chunk)
    pub fn get_row_chunk(&self, start_row: usize, num_rows: usize) -> Result<Matrix, String> {
        if start_row + num_rows > self.rows {
            return Err(format!(
                "Row chunk out of bounds: start={}, num_rows={}, total_rows={}",
                start_row, num_rows, self.rows
            ));
        }

        let mut chunk_data = Vec::with_capacity(num_rows * self.cols);
        for row in start_row..start_row + num_rows {
            let row_data = self.get_row(row)?;
            chunk_data.extend_from_slice(row_data);
        }

        Ok(Matrix {
            data: chunk_data,
            rows: num_rows,
            cols: self.cols,
        })
    }

    /// Get a submatrix (column chunk)
    pub fn get_col_chunk(&self, start_col: usize, num_cols: usize) -> Result<Matrix, String> {
        if start_col + num_cols > self.cols {
            return Err(format!(
                "Column chunk out of bounds: start={}, num_cols={}, total_cols={}",
                start_col, num_cols, self.cols
            ));
        }

        let mut chunk_data = Vec::with_capacity(self.rows * num_cols);
        for row in 0..self.rows {
            for col in start_col..start_col + num_cols {
                chunk_data.push(self.data[row * self.cols + col]);
            }
        }

        Ok(Matrix {
            data: chunk_data,
            rows: self.rows,
            cols: num_cols,
        })
    }

    /// Multiply two matrices (A * B)
    /// Returns a new matrix C where C[i][j] = sum(A[i][k] * B[k][j])
    pub fn multiply(&self, other: &Matrix) -> Result<Matrix, String> {
        if self.cols != other.rows {
            return Err(format!(
                "Matrix dimensions incompatible: {}x{} * {}x{}",
                self.rows, self.cols, other.rows, other.cols
            ));
        }

        let mut result = Matrix::new(self.rows, other.cols);

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self.data[i * self.cols + k] * other.data[k * other.cols + j];
                }
                result.data[i * other.cols + j] = sum;
            }
        }

        Ok(result)
    }

    /// Multiply a row chunk with a column chunk
    /// Used for distributed multiplication
    pub fn multiply_chunks(row_chunk: &Matrix, col_chunk: &Matrix) -> Result<Matrix, String> {
        if row_chunk.cols != col_chunk.rows {
            return Err(format!(
                "Chunk dimensions incompatible: {}x{} * {}x{}",
                row_chunk.rows, row_chunk.cols, col_chunk.rows, col_chunk.cols
            ));
        }

        let mut result = Matrix::new(row_chunk.rows, col_chunk.cols);

        for i in 0..row_chunk.rows {
            for j in 0..col_chunk.cols {
                let mut sum = 0.0;
                for k in 0..row_chunk.cols {
                    sum += row_chunk.data[i * row_chunk.cols + k]
                        * col_chunk.data[k * col_chunk.cols + j];
                }
                result.data[i * col_chunk.cols + j] = sum;
            }
        }

        Ok(result)
    }
}

