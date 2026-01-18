use distribiuted_matrix_multiplication::coordinator::Coordinator;
use distribiuted_matrix_multiplication::worker::Worker;
use mpi::traits::*;
use std::env;
use std::path::PathBuf;

fn main() {
    let universe = mpi::initialize().expect("Failed to initialize MPI");
    let world = universe.world();

    let rank = world.rank();
    let size = world.size();

    if size < 1 {
        eprintln!("Error: Need at least 1 process");
        eprintln!("Usage: <matrix_a> <matrix_b> <output>");
        std::process::exit(1);
    }
    
    if rank == 0 {
        let args: Vec<String> = env::args().collect();
        if args.len() != 4 {
            eprintln!("Usage: {} <matrix_a> <matrix_b> <output>", args[0]);
            eprintln!("  matrix_a: Path to first matrix file (text format)");
            eprintln!("  matrix_b: Path to second matrix file (text format)");
            eprintln!("  output:   Path to output matrix file (text format)");
            std::process::exit(1);
        }

        let matrix_a_path = PathBuf::from(&args[1]);
        let matrix_b_path = PathBuf::from(&args[2]);
        let output_path = PathBuf::from(&args[3]);

        println!("[Coordinator] Starting with {} workers", size - 1);
        println!("[Coordinator] Matrix A: {:?}", matrix_a_path);
        println!("[Coordinator] Matrix B: {:?}", matrix_b_path);
        println!("[Coordinator] Output: {:?}", output_path);

        let coordinator = Coordinator::new(world);
        if let Err(e) = coordinator.multiply_matrices(&matrix_a_path, &matrix_b_path, &output_path) {
            eprintln!("[Coordinator] Error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Worker process (including rank 0 in worker-only mode)
        let worker = Worker::new(world);
        if let Err(e) = worker.process_work() {
            eprintln!("[Worker {}] Error: {}", worker.rank(), e);
            std::process::exit(1);
        }
    }
}
