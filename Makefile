.PHONY: help build run test fmt lint audit deny clean docker-build docker-run k8s-deploy-coordinator k8s-deploy-workers k8s-e2e-test

help:
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the project
	cargo build --release

run: ## Run the application
	cargo run

test: ## Run tests
	cargo test

fmt: ## Format code
	cargo fmt --all

lint: ## Run clippy
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

generate-matrices: ## Generate test matrices (usage: make generate-matrices SIZE=1000 OUTPUT_A=matrix_a.txt OUTPUT_B=matrix_b.txt)
	python3 scripts/generate_matrices.py $(if $(SIZE),$(SIZE),) $(if $(OUTPUT_A),$(OUTPUT_A),matrix_a.txt) $(if $(OUTPUT_B),$(OUTPUT_B),matrix_b.txt)

verify-multiplication: ## Verify matrix multiplication result (usage: make verify-multiplication MATRIX_A=matrix_a.txt MATRIX_B=matrix_b.txt RESULT=output.txt TOLERANCE=1e-5)
	python3 scripts/verify_multiplication.py $(if $(MATRIX_A),$(MATRIX_A),matrix_a.txt) $(if $(MATRIX_B),$(MATRIX_B),matrix_b.txt) $(if $(RESULT),$(RESULT),output.txt) $(if $(TOLERANCE),$(TOLERANCE),)

k8s-e2e-test: ## End-to-end test (usage: make k8s-e2e-test WORKERS=4 MATRIX_SIZE=100)
	./scripts/e2e_test.sh $(if $(WORKERS),$(WORKERS),4) $(if $(MATRIX_SIZE),$(MATRIX_SIZE),100)
