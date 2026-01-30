.PHONY: help build run test fmt lint audit deny clean docker-build docker-run k8s-deploy-coordinator k8s-deploy-workers k8s-e2e-test

help:
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build:
	cargo build --release

run:
	cargo run

test:
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo clippy -- -D warnings

audit:
	cargo audit

deny:
	cargo deny check

clean:
	cargo clean

docker-build: 
	docker build -t distribiuted-matrix-multiplication:latest .

docker-run: 
	docker run --rm distribiuted-matrix-multiplication:latest

generate-matrices:
	python3 scripts/generate_matrices.py $(if $(SIZE),$(SIZE),) $(if $(OUTPUT_A),$(OUTPUT_A),matrix_a.txt) $(if $(OUTPUT_B),$(OUTPUT_B),matrix_b.txt)

verify-multiplication:
	python3 scripts/verify_multiplication.py $(if $(MATRIX_A),$(MATRIX_A),matrix_a.txt) $(if $(MATRIX_B),$(MATRIX_B),matrix_b.txt) $(if $(RESULT),$(RESULT),output.txt) $(if $(TOLERANCE),$(TOLERANCE),)

k8s-e2e-test:
   ./scripts/e2e_test.sh $(if $(WORKER_COUNT),$(WORKER_COUNT),4) $(if $(MATRIX_SIZE),$(MATRIX_SIZE),100)
