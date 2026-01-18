mod common;

use distribiuted_matrix_multiplication::matrix::Matrix;
use common::mpi_mock::{simulate_worker, test_mpi, TestMessageQueue};

#[test]
fn test_coordinator_worker_communication() {
    let queue = TestMessageQueue::new();

    let matrix_a = Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    let matrix_b = Matrix::from_vec(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], 3, 2).unwrap();

    let coordinator_rank = 0;
    let worker_rank = 1;

    test_mpi::send_work_assignment(&queue, coordinator_rank, worker_rank, 0, 2, 0, 2);

    let row_chunk = matrix_a.get_row_chunk(0, 2).unwrap();
    test_mpi::send_row_chunk(&queue, coordinator_rank, worker_rank, &row_chunk);

    let col_chunk = matrix_b.get_col_chunk(0, 2).unwrap();
    assert_eq!(col_chunk.rows, 3, "Column chunk should have 3 rows");
    assert_eq!(col_chunk.cols, 2, "Column chunk should have 2 columns");
    test_mpi::send_col_chunk(&queue, coordinator_rank, worker_rank, &col_chunk);

    let result = simulate_worker(&queue, worker_rank, coordinator_rank).unwrap();

    assert_eq!(result.rows, 2, "Result should have 2 rows");
    assert_eq!(result.cols, 2, "Result should have 2 columns");

    // Expected: A[0:2, :] * B[:, 0:2]
    // [1 2 3]   [7  8 ]
    // [4 5 6] * [9  10] = [58  64 ]
    //            [11 12]   [139 154]
    let expected = matrix_a.multiply(&matrix_b).unwrap();
    assert_eq!(result.get(0, 0).unwrap(), expected.get(0, 0).unwrap());
    assert_eq!(result.get(0, 1).unwrap(), expected.get(0, 1).unwrap());
    assert_eq!(result.get(1, 0).unwrap(), expected.get(1, 0).unwrap());
    assert_eq!(result.get(1, 1).unwrap(), expected.get(1, 1).unwrap());
}

#[test]
fn test_distributed_multiplication_with_multiple_workers() {
    let queue = TestMessageQueue::new();

    // Create larger test matrices: 4x3 and 3x4
    let a_data: Vec<f64> = (1..=12).map(|x| x as f64).collect();
    let b_data: Vec<f64> = (1..=12).map(|x| x as f64).collect();
    let matrix_a = Matrix::from_vec(a_data, 4, 3).unwrap();
    let matrix_b = Matrix::from_vec(b_data, 3, 4).unwrap();

    let coordinator_rank = 0;

    // Simulate coordinator distributing work to 2 workers
    // Worker 1: rows [0, 2), cols [0, 2)
    // Worker 2: rows [2, 4), cols [2, 4)

    test_mpi::send_work_assignment(&queue, coordinator_rank, 1, 0, 2, 0, 2);
    let row_chunk_1 = matrix_a.get_row_chunk(0, 2).unwrap();
    test_mpi::send_row_chunk(&queue, coordinator_rank, 1, &row_chunk_1);
    let col_chunk_1 = matrix_b.get_col_chunk(0, 2).unwrap();
    test_mpi::send_col_chunk(&queue, coordinator_rank, 1, &col_chunk_1);

    test_mpi::send_work_assignment(&queue, coordinator_rank, 2, 2, 4, 2, 4);
    let row_chunk_2 = matrix_a.get_row_chunk(2, 2).unwrap();
    test_mpi::send_row_chunk(&queue, coordinator_rank, 2, &row_chunk_2);
    let col_chunk_2 = matrix_b.get_col_chunk(2, 2).unwrap();
    test_mpi::send_col_chunk(&queue, coordinator_rank, 2, &col_chunk_2);

    let result_1 = simulate_worker(&queue, 1, coordinator_rank).unwrap();
    let result_2 = simulate_worker(&queue, 2, coordinator_rank).unwrap();

    let expected = matrix_a.multiply(&matrix_b).unwrap();

    for i in 0..result_1.rows {
        for j in 0..result_1.cols {
            assert_eq!(
                result_1.get(i, j).unwrap(),
                expected.get(i, j).unwrap(),
                "Worker 1 mismatch at ({}, {})",
                i,
                j
            );
        }
    }

    for i in 0..result_2.rows {
        for j in 0..result_2.cols {
            assert_eq!(
                result_2.get(i, j).unwrap(),
                expected.get(i + 2, j + 2).unwrap(),
                "Worker 2 mismatch at ({}, {})",
                i,
                j
            );
        }
    }
}

#[test]
fn test_worker_with_no_work() {
    let queue = TestMessageQueue::new();
    let coordinator_rank = 0;
    let worker_rank = 1;

    test_mpi::send_work_assignment(&queue, coordinator_rank, worker_rank, 5, 5, 5, 5);

    let result = simulate_worker(&queue, worker_rank, coordinator_rank);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No work assigned"));
}

#[test]
fn test_matrix_communication_roundtrip() {
    let queue = TestMessageQueue::new();
    let from = 0;
    let to = 1;

    let original = Matrix::from_vec(
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        3,
        3,
    )
    .unwrap();

    test_mpi::send_matrix(&queue, from, to, &original);

    let received = test_mpi::receive_matrix(&queue, from, to).unwrap();

    assert_eq!(original.rows, received.rows);
    assert_eq!(original.cols, received.cols);
    assert_eq!(original.data, received.data);
}

