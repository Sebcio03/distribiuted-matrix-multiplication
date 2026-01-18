#!/usr/bin/env python3
import sys
import numpy as np


def load_matrix(file_path):
    matrix = []
    with open(file_path, 'r') as f:
        for line in f:
            line = line.strip()
            if line:
                row = [float(x) for x in line.split()]
                matrix.append(row)
    return np.array(matrix, dtype=np.float32)


def verify_multiplication(matrix_a_path, matrix_b_path, result_path, tolerance=1e-5):
    A = load_matrix(matrix_a_path)
    B = load_matrix(matrix_b_path)
    C_result = load_matrix(result_path)
    C_expected = np.dot(A, B)
    
    diff = np.abs(C_result - C_expected)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    
    if np.allclose(C_result, C_expected, rtol=tolerance, atol=tolerance):
        print("PASSED: Result matches expected value!")
        return True
    else:
        num_errors = np.sum(diff > tolerance)
        total_elements = C_result.size
        error_percentage = (num_errors / total_elements) * 100
        print(f"FAILED: {num_errors}/{total_elements} elements ({error_percentage:.2f}%) differ by more than {tolerance}")
        return False


def main():
    if len(sys.argv) < 4 or len(sys.argv) > 5:
        print(f"Usage: {sys.argv[0]} <matrix_a> <matrix_b> <result_matrix> [tolerance]", file=sys.stderr)
        print(f"  matrix_a: Path to matrix A file", file=sys.stderr)
        print(f"  matrix_b: Path to matrix B file", file=sys.stderr)
        print(f"  result_matrix: Path to result matrix C file", file=sys.stderr)
        print(f"  tolerance: Floating point tolerance for comparison (default: 1e-5)", file=sys.stderr)
        print("", file=sys.stderr)
        print(f"Example: {sys.argv[0]} matrix_a.txt matrix_b.txt output.txt", file=sys.stderr)
        print(f"Example: {sys.argv[0]} matrix_a.txt matrix_b.txt output.txt 0.001", file=sys.stderr)
        sys.exit(1)
    
    matrix_a_path = sys.argv[1]
    matrix_b_path = sys.argv[2]
    result_path = sys.argv[3]
    tolerance = float(sys.argv[4]) if len(sys.argv) == 5 else 1e-5
    
    success = verify_multiplication(matrix_a_path, matrix_b_path, result_path, tolerance)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()



