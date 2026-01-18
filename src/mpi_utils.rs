use crate::matrix::Matrix;
use mpi::traits::*;

// MPI message tags
pub const TAG_MATRIX_DIMENSIONS: i32 = 1;
pub const TAG_MATRIX_DATA: i32 = 2;
pub const TAG_RESULT_DATA: i32 = 3;
pub const TAG_WORK_ASSIGNMENT: i32 = 4;

/// Send matrix dimensions to a destination
pub fn send_matrix_dimensions(
    world: &dyn Communicator,
    dest: i32,
    rows: usize,
    cols: usize,
) -> Result<(), String> {
    let dims = [rows as i32, cols as i32];
    let dest_process = world.process_at_rank(dest);
    dest_process.send_with_tag(&dims[..], TAG_MATRIX_DIMENSIONS);
    Ok(())
}

/// Receive matrix dimensions from a source
pub fn receive_matrix_dimensions(
    world: &dyn Communicator,
    source: i32,
) -> Result<(usize, usize), String> {
    let source_process = world.process_at_rank(source);
    let mut msg = [0i32; 2];
    source_process.receive_into_with_tag(&mut msg[..], TAG_MATRIX_DIMENSIONS);

    Ok((msg[0] as usize, msg[1] as usize))
}

/// Send a matrix to a destination
pub fn send_matrix(
    world: &dyn Communicator,
    dest: i32,
    matrix: &Matrix,
) -> Result<(), String> {
    // First send dimensions
    send_matrix_dimensions(world, dest, matrix.rows, matrix.cols)?;

    // Then send data
    let dest_process = world.process_at_rank(dest);
    dest_process.send_with_tag(&matrix.data[..], TAG_MATRIX_DATA);

    Ok(())
}

/// Receive a matrix from a source
pub fn receive_matrix(
    world: &dyn Communicator,
    source: i32,
) -> Result<Matrix, String> {
    // First receive dimensions
    let (rows, cols) = receive_matrix_dimensions(world, source)?;

    // Then receive data
    let source_process = world.process_at_rank(source);
    let mut data = vec![0.0f64; rows * cols];
    source_process.receive_into_with_tag(&mut data[..], TAG_MATRIX_DATA);

    Ok(Matrix {
        data,
        rows,
        cols,
    })
}

/// Broadcast matrix dimensions to all processes
pub fn broadcast_dimensions(
    world: &dyn Communicator,
    root: i32,
    rows: usize,
    cols: usize,
) -> Result<(usize, usize), String> {
    let root_process = world.process_at_rank(root);
    let mut dims = if world.rank() == root {
        vec![rows as i32, cols as i32]
    } else {
        vec![0i32; 2]
    };

    root_process.broadcast_into(&mut dims[..]);

    Ok((dims[0] as usize, dims[1] as usize))
}

/// Send work assignment (row range and column range) to a worker
pub fn send_work_assignment(
    world: &dyn Communicator,
    dest: i32,
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
) -> Result<(), String> {
    let assignment = [
        row_start as i32,
        row_end as i32,
        col_start as i32,
        col_end as i32,
    ];
    let dest_process = world.process_at_rank(dest);
    dest_process.send_with_tag(&assignment[..], TAG_WORK_ASSIGNMENT);
    Ok(())
}

/// Receive work assignment from coordinator
pub fn receive_work_assignment(
    world: &dyn Communicator,
    source: i32,
) -> Result<(usize, usize, usize, usize), String> {
    let source_process = world.process_at_rank(source);
    let mut assignment = [0i32; 4];
    source_process.receive_into_with_tag(&mut assignment[..], TAG_WORK_ASSIGNMENT);

    Ok((
        assignment[0] as usize,
        assignment[1] as usize,
        assignment[2] as usize,
        assignment[3] as usize,
    ))
}

/// Send result matrix chunk to coordinator
pub fn send_result(
    world: &dyn Communicator,
    dest: i32,
    result: &Matrix,
) -> Result<(), String> {
    send_matrix(world, dest, result)
}

/// Receive result matrix chunk from worker
pub fn receive_result(
    world: &dyn Communicator,
    source: i32,
) -> Result<Matrix, String> {
    receive_matrix(world, source)
}

