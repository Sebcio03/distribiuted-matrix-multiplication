#!/bin/bash
set -e

echo "=== Setting up local Kubernetes environment for MPIJob testing ==="
echo ""

# Check if Docker is running
if ! docker ps >/dev/null 2>&1; then
    echo "ERROR: Docker is not running!"
    echo "Please start Docker Desktop or Docker daemon first."
    exit 1
fi
echo "✓ Docker is running"

# Check if minikube is installed
if ! command -v minikube &> /dev/null; then
    echo "ERROR: minikube is not installed!"
    echo "Install it with: brew install minikube"
    exit 1
fi

# Check minikube status
MINIKUBE_STATUS=$(minikube status --format={{.Host}} 2>/dev/null || echo "Nonexistent")

if [ "$MINIKUBE_STATUS" != "Running" ]; then
    echo ""
    echo "Starting minikube..."
    minikube start
    
    if [ $? -ne 0 ]; then
        echo "ERROR: Failed to start minikube"
        exit 1
    fi
    echo "✓ Minikube started"
else
    echo "✓ Minikube is already running"
fi

# Set up Docker environment for minikube
echo ""
echo "Setting up Docker environment for minikube..."
eval $(minikube docker-env)

# Check if MPI Operator is installed
echo ""
echo "Checking for MPI Operator..."
if ! kubectl get crd mpijobs.kubeflow.org >/dev/null 2>&1; then
    echo "MPI Operator not found. Installing..."
    kubectl apply --server-side -f https://raw.githubusercontent.com/kubeflow/mpi-operator/v0.7.0/deploy/v2beta1/mpi-operator.yaml
    
    echo "Waiting for MPI Operator to be ready..."
    kubectl wait --for=condition=ready pod -l app=mpi-operator -n mpi-operator --timeout=120s || {
        echo "WARNING: MPI Operator may not be fully ready, but continuing..."
    }
    echo "✓ MPI Operator installed"
else
    echo "✓ MPI Operator found"
fi

# Build Docker image in minikube's Docker environment
echo ""
echo "Building Docker image in minikube environment..."
docker build -t distribiuted-matrix-multiplication:latest .

if [ $? -ne 0 ]; then
    echo "ERROR: Docker build failed"
    exit 1
fi
echo "✓ Docker image built successfully"

# Verify image exists
if docker images | grep -q "distribiuted-matrix-multiplication.*latest"; then
    echo "✓ Image verified in minikube Docker environment"
else
    echo "WARNING: Image not found after build"
fi

echo ""
echo "=== Setup complete! ==="
echo ""
echo "You can now run the e2e test with:"
echo "  ./scripts/e2e_test.sh [WORKER_COUNT] [MATRIX_SIZE]"
echo ""
echo "Or run it now? (y/n)"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    WORKER_COUNT=${1:-4}
    MATRIX_SIZE=${2:-100}
    echo ""
    echo "Running e2e test with $WORKER_COUNT workers and ${MATRIX_SIZE}x${MATRIX_SIZE} matrices..."
    ./scripts/e2e_test.sh "$WORKER_COUNT" "$MATRIX_SIZE"
fi
