#!/usr/bin/env python3
import sys
import os


def generate_random_value(i, j):
    # Use a deterministic hash function that doesn't depend on Python's hash randomization
    hash_val = ((i * 2654435761) ^ (j * 2246822519)) % 1000000
    return hash_val / 10000.0


def generate_matrix(size, output_path):
    """Generate a matrix and save it to a file."""
    print(f"Generating {size}x{size} matrix...")
    
    with open(output_path, 'w') as f:
        for i in range(size):
            row = []
            for j in range(size):
                value = generate_random_value(i, j)
                row.append(str(value))
            f.write(' '.join(row) + '\n')
    
    file_size = os.path.getsize(output_path)
    print(f"Matrix saved: {file_size} bytes ({file_size / (1024 * 1024):.2f} MB)")


def main():
    # Default: generate 500MB matrices (~11,445 x 11,445)
    default_size = 11445
    if len(sys.argv) == 4:
        size = int(sys.argv[1])
        output_a = sys.argv[2]
        output_b = sys.argv[3]
    elif len(sys.argv) == 3:
        size = default_size
        output_a = sys.argv[1]
        output_b = sys.argv[2]
    else:
        print(f"Usage: {sys.argv[0]} [size] <matrix_a_output> <matrix_b_output>", file=sys.stderr)
        print(f"  size: Matrix dimensions (default: {default_size} for ~500MB)", file=sys.stderr)
        print(f"  matrix_a_output: Path to output file for matrix A", file=sys.stderr)
        print(f"  matrix_b_output: Path to output file for matrix B", file=sys.stderr)
        print("", file=sys.stderr)
        print(f"Example: {sys.argv[0]} matrix_a.txt matrix_b.txt", file=sys.stderr)
        print(f"Example: {sys.argv[0]} 1000 matrix_a.txt matrix_b.txt", file=sys.stderr)
        sys.exit(1)
    
    print(f"Generating two {size}x{size} matrices (~{(size * size * 4) / (1024 * 1024)}MB each)...")
    print(f"Matrix A: {output_a}")
    print(f"Matrix B: {output_b}")
    
    # Generate matrix A
    print("Generating matrix A...")
    generate_matrix(size, output_a)
    
    # Generate matrix B
    print("Generating matrix B...")
    generate_matrix(size, output_b)
    
    print("exit")


if __name__ == "__main__":
    main()



