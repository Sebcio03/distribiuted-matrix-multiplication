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

echo ""
echo "3. Waiting for pods to be created..."
sleep 10

# Wait for pods to be created and show their status
for i in {1..20}; do
    echo "Checking pod status (attempt $i/20)..."
    kubectl get pods -l app=distribiuted-matrix-multiplication 2>&1 || true
    
    # Check if launcher pod exists
    LAUNCHER_POD=$(kubectl get pods -l role=launcher,app=distribiuted-matrix-multiplication -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
    if [ -n "$LAUNCHER_POD" ]; then
        echo ""
        echo "Launcher pod: $LAUNCHER_POD"
        kubectl get pod "$LAUNCHER_POD" -o wide
        echo ""
        echo "Launcher pod events:"
        kubectl describe pod "$LAUNCHER_POD" | grep -A 20 "Events:" || true
    fi
    
    # Check if worker pods exist
    WORKER_PODS=$(kubectl get pods -l role=worker,app=distribiuted-matrix-multiplication -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
    if [ -n "$WORKER_PODS" ]; then
        echo ""
        echo "Worker pods found: $WORKER_PODS"
        for worker in $WORKER_PODS; do
            echo "Worker pod $worker:"
            kubectl get pod "$worker" -o wide
        done
    fi
    
    # Check MPIJob status
    echo ""
    echo "MPIJob status:"
    kubectl get mpijob matrix-multiplication-mpijob -o yaml | grep -A 10 "status:" || kubectl get mpijob matrix-multiplication-mpijob
    
    # If pods are running, break early
    if [ -n "$LAUNCHER_POD" ] && [ -n "$WORKER_PODS" ]; then
        LAUNCHER_STATUS=$(kubectl get pod "$LAUNCHER_POD" -o jsonpath='{.status.phase}' 2>/dev/null || echo "")
        if [ "$LAUNCHER_STATUS" = "Running" ] || [ "$LAUNCHER_STATUS" = "Succeeded" ] || [ "$LAUNCHER_STATUS" = "Failed" ]; then
            echo "Pods are in final state, proceeding..."
            break
        fi
    fi
    
    sleep 5
done

echo ""
echo "4. Waiting for MPIJob to complete (timeout: 600s)..."
if ! kubectl wait --for=condition=complete mpijob/matrix-multiplication-mpijob --timeout=600s 2>&1; then
    echo ""
    echo "=== MPIJob TIMEOUT or FAILED ==="
    echo ""
    echo "MPIJob status:"
    kubectl get mpijob matrix-multiplication-mpijob -o yaml | grep -A 30 "status:" || kubectl get mpijob matrix-multiplication-mpijob
    
    echo ""
    echo "All pods status:"
    kubectl get pods -l app=distribiuted-matrix-multiplication
    
    echo ""
    echo "=== Launcher pod details ==="
    LAUNCHER_POD=$(kubectl get pods -l role=launcher,app=distribiuted-matrix-multiplication -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
    if [ -n "$LAUNCHER_POD" ]; then
        echo "Launcher pod: $LAUNCHER_POD"
        kubectl describe pod "$LAUNCHER_POD"
        echo ""
        echo "=== Launcher logs ==="
        kubectl logs "$LAUNCHER_POD" --tail=200 2>&1 || echo "Could not get launcher logs"
    else
        echo "No launcher pod found!"
        echo "Attempting to get logs by label:"
        kubectl logs -l role=launcher,app=distribiuted-matrix-multiplication --tail=200 2>&1 || echo "Could not get launcher logs"
    fi
    
    echo ""
    echo "=== Worker pods details ==="
    WORKER_PODS=$(kubectl get pods -l role=worker,app=distribiuted-matrix-multiplication -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
    if [ -n "$WORKER_PODS" ]; then
        for worker in $WORKER_PODS; do
            echo "=== Worker $worker ==="
            kubectl describe pod "$worker" | tail -40
            echo ""
            echo "Logs:"
            kubectl logs "$worker" --tail=100 2>&1 || true
            echo ""
        done
    else
        echo "No worker pods found!"
        echo "Attempting to get logs by label:"
        kubectl logs -l role=worker,app=distribiuted-matrix-multiplication --tail=50 2>&1 | head -100 || echo "Could not get worker logs"
    fi
    
    exit 1
fi

echo ""
echo "5. Checking results..."
LAUNCHER_POD=$(kubectl get pods -l role=launcher,app=distribiuted-matrix-multiplication -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
if [ -n "$LAUNCHER_POD" ]; then
    LOGS=$(kubectl logs "$LAUNCHER_POD" 2>&1)
else
    LOGS=$(kubectl logs -l role=launcher,app=distribiuted-matrix-multiplication 2>&1)
fi
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
