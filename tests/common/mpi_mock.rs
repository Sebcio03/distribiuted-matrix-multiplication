
use distribiuted_matrix_multiplication::matrix::Matrix;
use distribiuted_matrix_multiplication::mpi_utils::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Message queue for simulating MPI communication in tests
#[derive(Clone)]
pub struct TestMessageQueue {
    messages: Arc<Mutex<HashMap<(i32, i32, i32), Vec<Vec<u8>>>>>,
}

impl TestMessageQueue {
    pub fn new() -> Self {
        TestMessageQueue {
            messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Send data from one rank to another
    pub fn send<T: Copy>(&self, from: i32, to: i32, tag: i32, data: &[T]) {
        let size = data.len() * std::mem::size_of::<T>();
        let bytes: Vec<u8> = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, size)
        }
        .to_vec();

        let mut msgs = self.messages.lock().unwrap();
        msgs.entry((from, to, tag))
            .or_insert_with(Vec::new)
            .push(bytes);
    }

    /// Receive data from one rank to another
    pub fn receive<T: Copy>(&self, from: i32, to: i32, tag: i32, buf: &mut [T]) -> bool {
        let mut msgs = self.messages.lock().unwrap();
        if let Some(msg_queue) = msgs.get_mut(&(from, to, tag)) {
            if let Some(msg_bytes) = msg_queue.pop() {
                let expected_size = buf.len() * std::mem::size_of::<T>();
                if msg_bytes.len() >= expected_size {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            msg_bytes.as_ptr(),
                            buf.as_mut_ptr() as *mut u8,
                            expected_size.min(msg_bytes.len()),
                        );
                    }
                    return true;
                }
            }
        }
        false
    }
}

/// Test-specific MPI utility functions
pub mod test_mpi {
    use super::*;

    pub fn send_matrix_dimensions(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
        rows: usize,
        cols: usize,
    ) {
        let dims = [rows as i32, cols as i32];
        queue.send(from, to, TAG_MATRIX_DIMENSIONS, &dims);
    }

    pub fn receive_matrix_dimensions(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
    ) -> Option<(usize, usize)> {
        let mut dims = [0i32; 2];
        if queue.receive(from, to, TAG_MATRIX_DIMENSIONS, &mut dims) {
            Some((dims[0] as usize, dims[1] as usize))
        } else {
            None
        }
    }

    pub fn send_matrix(queue: &TestMessageQueue, from: i32, to: i32, matrix: &Matrix) {
        send_matrix_dimensions(queue, from, to, matrix.rows, matrix.cols);
        queue.send(from, to, TAG_MATRIX_DATA, &matrix.data);
    }

    pub fn receive_matrix(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
    ) -> Option<Matrix> {
        let (rows, cols) = receive_matrix_dimensions(queue, from, to)?;
        let mut data = vec![0.0f64; rows * cols];
        if queue.receive(from, to, TAG_MATRIX_DATA, &mut data) {
            Some(Matrix { data, rows, cols })
        } else {
            None
        }
    }

    pub fn send_work_assignment(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
        row_start: usize,
        row_end: usize,
        col_start: usize,
        col_end: usize,
    ) {
        let assignment = [
            row_start as i32,
            row_end as i32,
            col_start as i32,
            col_end as i32,
        ];
        queue.send(from, to, TAG_WORK_ASSIGNMENT, &assignment);
    }

    pub fn receive_work_assignment(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
    ) -> Option<(usize, usize, usize, usize)> {
        let mut assignment = [0i32; 4];
        if queue.receive(from, to, TAG_WORK_ASSIGNMENT, &mut assignment) {
            Some((
                assignment[0] as usize,
                assignment[1] as usize,
                assignment[2] as usize,
                assignment[3] as usize,
            ))
        } else {
            None
        }
    }

    // Separate functions for row chunk and column chunk to avoid tag conflicts
    pub fn send_row_chunk(queue: &TestMessageQueue, from: i32, to: i32, matrix: &Matrix) {
        send_matrix_dimensions(queue, from, to, matrix.rows, matrix.cols);
        queue.send(from, to, TAG_MATRIX_DATA, &matrix.data);
    }

    pub fn send_col_chunk(queue: &TestMessageQueue, from: i32, to: i32, matrix: &Matrix) {
        // Use tag 10 for col chunk dimensions to avoid mixing with row chunk
        let dims = [matrix.rows as i32, matrix.cols as i32];
        queue.send(from, to, 10, &dims);
        // Use TAG_RESULT_DATA for col chunk data
        queue.send(from, to, TAG_RESULT_DATA, &matrix.data);
    }

    pub fn receive_row_chunk(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
    ) -> Option<Matrix> {
        let (rows, cols) = receive_matrix_dimensions(queue, from, to)?;
        let mut data = vec![0.0f64; rows * cols];
        if queue.receive(from, to, TAG_MATRIX_DATA, &mut data) {
            Some(Matrix { data, rows, cols })
        } else {
            None
        }
    }

    pub fn receive_col_chunk(
        queue: &TestMessageQueue,
        from: i32,
        to: i32,
    ) -> Option<Matrix> {
        // Receive dimensions with tag 10
        let mut dims = [0i32; 2];
        if !queue.receive(from, to, 10, &mut dims) {
            return None;
        }
        let rows = dims[0] as usize;
        let cols = dims[1] as usize;
        let mut data = vec![0.0f64; rows * cols];
        // Receive data with TAG_RESULT_DATA
        if queue.receive(from, to, TAG_RESULT_DATA, &mut data) {
            Some(Matrix { data, rows, cols })
        } else {
            None
        }
    }
}

/// Simulate a worker processing work
pub fn simulate_worker(
    queue: &TestMessageQueue,
    worker_rank: i32,
    coordinator_rank: i32,
) -> Result<Matrix, String> {
    // Receive work assignment
    let (row_start, row_end, col_start, col_end) = test_mpi::receive_work_assignment(
        queue,
        coordinator_rank,
        worker_rank,
    )
    .ok_or("Failed to receive work assignment")?;

    if row_start >= row_end || col_start >= col_end {
        return Err("No work assigned".to_string());
    }

    // Receive row chunk from A
    let row_chunk = test_mpi::receive_row_chunk(queue, coordinator_rank, worker_rank)
        .ok_or("Failed to receive row chunk")?;

    // Receive column chunk from B
    let col_chunk = test_mpi::receive_col_chunk(queue, coordinator_rank, worker_rank)
        .ok_or("Failed to receive column chunk")?;

    // Compute result
    let result = Matrix::multiply_chunks(&row_chunk, &col_chunk)?;

    // Send result back
    test_mpi::send_matrix(queue, worker_rank, coordinator_rank, &result);

    Ok(result)
}

