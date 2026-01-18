#!/bin/bash
set -e

export WORKER_COUNT=${1:-4}
export MATRIX_SIZE=${2:-100}

echo "=== E2E Test: $WORKER_COUNT workers, ${MATRIX_SIZE}x${MATRIX_SIZE} matrices ==="
echo ""

echo "1. Checking for Kubeflow MPI Operator..."
if ! kubectl get crd mpijobs.kubeflow.org >/dev/null 2>&1; then
    echo "ERROR: Kubeflow MPI Operator not found!"
    echo "Please install it with: kubectl apply --server-side -f https://raw.githubusercontent.com/kubeflow/mpi-operator/v0.7.0/deploy/v2beta1/mpi-operator.yaml"
    exit 1
fi
echo "MPI Operator found ✓"

echo ""
echo "2. Running MPIJob..."
kubectl delete mpijob matrix-multiplication-mpijob --ignore-not-found=true
sed "s/\${WORKER_COUNT}/${WORKER_COUNT}/g; s/\${MATRIX_SIZE}/${MATRIX_SIZE}/g" k8s/mpijob.yaml | kubectl apply -f -

echo "Waiting for MPIJob to complete..."
if ! kubectl wait --for=condition=complete mpijob/matrix-multiplication-mpijob --timeout=300s; then
    echo "MPIJob failed. Launcher logs:"
    kubectl logs -l role=launcher,app=distribiuted-matrix-multiplication --tail=100
    echo ""
    echo "Worker logs (first worker):"
    kubectl logs -l role=worker,app=distribiuted-matrix-multiplication --tail=50 | head -100
    exit 1
fi

echo ""
echo "3. Checking results..."
LOGS=$(kubectl logs -l role=launcher,app=distribiuted-matrix-multiplication)
echo "$LOGS"

echo ""
if echo "$LOGS" | grep -q "VERIFICATION PASSED"; then
    echo "=== E2E Test PASSED ==="
elif echo "$LOGS" | grep -q "Done! Output saved"; then
    echo "=== E2E Test PASSED (verification skipped) ==="
else
    echo "=== E2E Test FAILED ==="
    exit 1
fi
