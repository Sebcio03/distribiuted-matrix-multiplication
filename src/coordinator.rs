use crate::matrix::Matrix;
use crate::mpi_utils::*;
use mpi::traits::*;
use std::path::Path;

pub struct Coordinator<C: Communicator> {
    world: C,
    worker_count: usize,
}

impl<C: Communicator> Coordinator<C> {
    /// Create a new coordinator
    pub fn new(world: C) -> Self {
        let size = world.size() as usize;
        let worker_count = if size > 1 { size - 1 } else { 0 };
        Coordinator {
            world,
            worker_count,
        }
    }

    /// Get the number of workers
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Multiply two matrices using distributed workers
    pub fn multiply_matrices(
        &self,
        matrix_a_path: &Path,
        matrix_b_path: &Path,
        output_path: &Path,
    ) -> Result<(), String> {
        let total_size = self.world.size() as usize;
        let actual_worker_count = if total_size > 1 { total_size - 1 } else { 0 };
        
        if actual_worker_count == 0 {
            return Err(format!(
                "No workers available. Need at least 2 processes (1 coordinator + 1 worker). Current size: {}",
                total_size
            ));
        }

        println!("[Coordinator] Loading matrices...");
        let matrix_a = Matrix::load_from_file(matrix_a_path)
            .map_err(|e| format!("Failed to load matrix A: {}", e))?;
        let matrix_b = Matrix::load_from_file(matrix_b_path)
            .map_err(|e| format!("Failed to load matrix B: {}", e))?;

        // Validate dimensions
        if matrix_a.cols != matrix_b.rows {
            return Err(format!(
                "Matrix dimensions incompatible: A is {}x{}, B is {}x{}",
                matrix_a.rows, matrix_a.cols, matrix_b.rows, matrix_b.cols
            ));
        }

        println!(
            "[Coordinator] Matrix A: {}x{}, Matrix B: {}x{}",
            matrix_a.rows, matrix_a.cols, matrix_b.rows, matrix_b.cols
        );

        // Broadcast dimensions to all workers (for synchronization)
        let (_a_rows, _a_cols) = broadcast_dimensions(&self.world, 0, matrix_a.rows, matrix_a.cols)?;
        let (_b_rows, _b_cols) = broadcast_dimensions(&self.world, 0, matrix_b.rows, matrix_b.cols)?;

        // Distribute work: split A by rows (1D row decomposition)
        // Each worker gets: rows [r1, r2) of A and the ENTIRE matrix B
        // Worker computes: result[r1:r2, :] = A[r1:r2, :] * B
        let rows_per_worker = (matrix_a.rows + actual_worker_count - 1) / actual_worker_count;

        println!(
            "[Coordinator] Distributing work: {} rows per worker (row-based decomposition)",
            rows_per_worker
        );

        // Send work to each worker
        for worker_rank in 1..total_size {
            let worker_rank_i32 = worker_rank as i32;

            // Calculate row range for this worker
            let row_start = (worker_rank - 1) * rows_per_worker;
            let row_end = (row_start + rows_per_worker).min(matrix_a.rows);

            if row_start >= matrix_a.rows {
                // No work for this worker - send empty assignment
                send_work_assignment(&self.world, worker_rank_i32, 0, 0, 0, 0)?;
                continue;
            }

            println!(
                "[Coordinator] Assigning to worker {}: rows [{}, {}), all columns",
                worker_rank, row_start, row_end
            );

            // Send work assignment (col range is full width: 0 to matrix_b.cols)
            send_work_assignment(&self.world, worker_rank_i32, row_start, row_end, 0, matrix_b.cols)?;

            // Send row chunk from A
            let row_chunk = matrix_a.get_row_chunk(row_start, row_end - row_start)?;
            send_matrix(&self.world, worker_rank_i32, &row_chunk)?;

            // Send entire matrix B to each worker
            send_matrix(&self.world, worker_rank_i32, &matrix_b)?;
        }

        // Initialize result matrix
        let mut result = Matrix::new(matrix_a.rows, matrix_b.cols);

        // Collect results from workers
        println!("[Coordinator] Collecting results from workers...");
        for worker_rank in 1..total_size {
            let worker_rank_i32 = worker_rank as i32;

            // Calculate expected row range
            let row_start = (worker_rank - 1) * rows_per_worker;

            if row_start >= matrix_a.rows {
                continue;
            }

            // Receive result chunk
            let result_chunk = receive_result(&self.world, worker_rank_i32)?;

            println!(
                "[Coordinator] Received result from worker {}: {}x{}",
                worker_rank, result_chunk.rows, result_chunk.cols
            );

            // Copy result chunk into final result matrix (full row width)
            for i in 0..result_chunk.rows {
                for j in 0..result_chunk.cols {
                    let global_row = row_start + i;
                    if global_row < result.rows && j < result.cols {
                        result.set(global_row, j, result_chunk.get(i, j)?)?;
                    }
                }
            }
        }

        // Save result
        println!("[Coordinator] Saving result to {:?}...", output_path);
        result.save_to_file(output_path)?;
        println!("[Coordinator] Multiplication complete!");

        Ok(())
    }
}
