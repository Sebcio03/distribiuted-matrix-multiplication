use crate::matrix::Matrix;
use crate::mpi_utils::*;
use mpi::traits::*;

pub struct Worker<C: Communicator> {
    rank: i32,
    world: C,
}

impl<C: Communicator> Worker<C> {
    /// Create a new worker
    pub fn new(world: C) -> Self {
        let rank = world.rank();
        Worker { rank, world }
    }

    /// Get the worker's rank
    pub fn rank(&self) -> i32 {
        self.rank
    }

    /// Process work assigned by the coordinator
    pub fn process_work(&self) -> Result<(), String> {
        println!("[Worker {}] Waiting for work assignment...", self.rank);

        // Receive work assignment from coordinator (rank 0)
        let (row_start, row_end, col_start, col_end) =
            receive_work_assignment(&self.world, 0)?;

        println!(
            "[Worker {}] Received assignment: rows [{}, {}), cols [{}, {})",
            self.rank, row_start, row_end, col_start, col_end
        );

        // Check if there's actual work to do
        if row_start >= row_end || col_start >= col_end {
            println!("[Worker {}] No work assigned, exiting", self.rank);
            return Ok(());
        }

        // Receive row chunk from matrix A
        println!("[Worker {}] Receiving row chunk from matrix A...", self.rank);
        let row_chunk = receive_matrix(&self.world, 0)?;
        println!(
            "[Worker {}] Received row chunk: {}x{}",
            self.rank, row_chunk.rows, row_chunk.cols
        );

        // Receive matrix B
        println!("[Worker {}] Receiving matrix B...", self.rank);
        let matrix_b = receive_matrix(&self.world, 0)?;
        println!(
            "[Worker {}] Received matrix B: {}x{}",
            self.rank, matrix_b.rows, matrix_b.cols
        );

        // Perform local matrix multiplication
        println!("[Worker {}] Computing multiplication...", self.rank);
        let result = Matrix::multiply_chunks(&row_chunk, &matrix_b)?;
        println!(
            "[Worker {}] Computed result: {}x{}",
            self.rank, result.rows, result.cols
        );

        // Send result back to coordinator
        println!("[Worker {}] Sending result to coordinator...", self.rank);
        send_result(&self.world, 0, &result)?;
        println!("[Worker {}] Work complete!", self.rank);

        Ok(())
    }
}

