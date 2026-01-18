.PHONY: help build run test fmt lint audit deny clean docker-build docker-run k8s-e2e-test k8s-check-mpi-operator k8s-install-mpi-operator

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

k8s-check-mpi-operator: ## Check if Kubeflow MPI Operator is installed
	@if kubectl get crd mpijobs.kubeflow.org >/dev/null 2>&1; then \
		echo "✓ Kubeflow MPI Operator is installed"; \
		kubectl get crd mpijobs.kubeflow.org; \
	else \
		echo "✗ Kubeflow MPI Operator is NOT installed"; \
		echo "  Install it with: make k8s-install-mpi-operator"; \
		exit 1; \
	fi

k8s-install-mpi-operator: ## Install Kubeflow MPI Operator
	@echo "Installing Kubeflow MPI Operator..."
	kubectl apply --server-side -f https://raw.githubusercontent.com/kubeflow/mpi-operator/v0.7.0/deploy/v2beta1/mpi-operator.yaml
	@echo "Waiting for MPI Operator to be ready..."
	@kubectl wait --for=condition=available deployment/mpi-operator -n mpi-operator --timeout=120s || true
	@echo "✓ MPI Operator installation complete"

k8s-e2e-test: ## End-to-end test (usage: make k8s-e2e-test WORKERS=4 MATRIX_SIZE=100)
	./scripts/e2e_test.sh $(if $(WORKERS),$(WORKERS),4) $(if $(MATRIX_SIZE),$(MATRIX_SIZE),100)
