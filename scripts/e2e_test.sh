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
echo "2. Checking Docker image availability..."
# Try to detect if we're using Minikube
if command -v minikube &> /dev/null && minikube status &> /dev/null; then
    echo "Minikube detected, checking image in Minikube..."
    eval $(minikube docker-env)
    if ! docker images | grep -q "distribiuted-matrix-multiplication.*latest"; then
        echo "WARNING: Image distribiuted-matrix-multiplication:latest not found in Minikube!"
        echo "Building image..."
        docker build -t distribiuted-matrix-multiplication:latest . || {
            echo "ERROR: Failed to build Docker image"
            exit 1
        }
    else
        echo "Image found in Minikube ✓"
    fi
else
    echo "Checking if image exists locally..."
    if ! docker images | grep -q "distribiuted-matrix-multiplication.*latest"; then
        echo "WARNING: Image distribiuted-matrix-multiplication:latest not found locally!"
        echo "You may need to build it: docker build -t distribiuted-matrix-multiplication:latest ."
    else
        echo "Image found locally ✓"
    fi
fi

echo ""
echo "3. Generating matrices locally..."
TMP_DIR=$(mktemp -d)
MATRIX_A="${TMP_DIR}/matrix_a.txt"
MATRIX_B="${TMP_DIR}/matrix_b.txt"
trap 'rm -rf "${TMP_DIR}"; kubectl delete configmap matrix-data-config --ignore-not-found=true >/dev/null 2>&1 || true' EXIT

echo "  Generating matrices locally..."
python3 scripts/generate_matrices.py "${MATRIX_SIZE}" "${MATRIX_A}" "${MATRIX_B}"

echo ""
echo "4. Creating ConfigMaps and MPIJob..."
# Apply MPI env ConfigMap
kubectl apply -f k8s/mpi-env-configmap.yaml

# Recreate matrix data ConfigMap from local files
kubectl delete configmap matrix-data-config --ignore-not-found=true
kubectl create configmap matrix-data-config \
    --from-file=matrix_a.txt="${MATRIX_A}" \
    --from-file=matrix_b.txt="${MATRIX_B}"

# Apply result PVC
kubectl apply -f k8s/matrix-storage-pvc.yaml
for i in {1..30}; do
    PVC_PHASE=$(kubectl get pvc matrix-storage -o jsonpath='{.status.phase}' 2>/dev/null || echo "")
    if [ "$PVC_PHASE" = "Bound" ]; then
        break
    fi
    echo "Waiting for PVC to be Bound... ($i/30)"
    sleep 2
done
if [ "$PVC_PHASE" != "Bound" ]; then
    echo "ERROR: PVC matrix-storage not Bound"
    kubectl get pvc matrix-storage -o wide || true
    exit 1
fi

# Delete existing MPIJob if present
kubectl delete mpijob matrix-multiplication-mpijob --ignore-not-found=true
for i in {1..30}; do
    if ! kubectl get mpijob matrix-multiplication-mpijob >/dev/null 2>&1; then
        break
    fi
    echo "Waiting for existing MPIJob to be deleted... ($i/30)"
    sleep 2
done

# Clean up any leftover pods from previous runs
kubectl delete pods -l training.kubeflow.org/job-name=matrix-multiplication-mpijob --ignore-not-found=true
for i in {1..30}; do
    EXISTING_PODS=$(kubectl get pods -l training.kubeflow.org/job-name=matrix-multiplication-mpijob -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
    if [ -z "$EXISTING_PODS" ]; then
        break
    fi
    echo "Waiting for old MPIJob pods to terminate... ($i/30)"
    sleep 2
done

# Create temporary file with substituted values
TMP_FILE=$(mktemp)
sed "s/\${WORKER_COUNT}/${WORKER_COUNT}/g" k8s/mpijob.yaml > "$TMP_FILE"

echo "Applied MPIJob configuration:"
echo "  WORKER_COUNT: $WORKER_COUNT"
echo ""

kubectl apply -f "$TMP_FILE"
rm "$TMP_FILE"

echo ""
echo "5. Waiting for pods to be created..."
# Wait for pods to be created and show their status
for i in {1..20}; do
    echo "Checking pod status (attempt $i/20)..."
    kubectl get pods -l training.kubeflow.org/job-name=matrix-multiplication-mpijob 2>&1 || true
    
    # Refresh launcher pod name (may change or terminate)
    LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --field-selector=status.phase!=Succeeded,status.phase!=Failed -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
    if [ -n "$LAUNCHER_POD" ]; then
        echo ""
        echo "Launcher pod: $LAUNCHER_POD"
        kubectl get pod "$LAUNCHER_POD" -o wide 2>&1 || true
        echo ""
        echo "Launcher pod events:"
        kubectl describe pod "$LAUNCHER_POD" | grep -A 20 "Events:" || true
    else
        echo ""
        echo "Launcher pod not found (may have completed)."
    fi
    
    # Check if worker pods exist
    WORKER_PODS=$(kubectl get pods -l role=worker,training.kubeflow.org/job-name=matrix-multiplication-mpijob -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
    if [ -n "$WORKER_PODS" ]; then
        echo ""
        echo "Worker pods found: $WORKER_PODS"
        for worker in $WORKER_PODS; do
            echo "Worker pod $worker:"
            kubectl get pod "$worker" -o wide 2>&1 || true
        done
    fi
    
    # Check MPIJob status
    echo ""
    echo "MPIJob status:"
    kubectl get mpijob matrix-multiplication-mpijob -o yaml | grep -A 10 "status:" || kubectl get mpijob matrix-multiplication-mpijob
    
    # If pods are running, break early
    if [ -n "$LAUNCHER_POD" ]; then
        LAUNCHER_STATUS=$(kubectl get pod "$LAUNCHER_POD" -o jsonpath='{.status.phase}' 2>/dev/null || echo "")
        if [ "$LAUNCHER_STATUS" = "Running" ] || [ "$LAUNCHER_STATUS" = "Succeeded" ] || [ "$LAUNCHER_STATUS" = "Failed" ]; then
            echo "Pods are in final state, proceeding..."
            break
        fi
    else
        echo "No active launcher pod; proceeding to MPIJob wait..."
        break
    fi
    
    sleep 5
done

echo ""
echo "6. Waiting for MPIJob to complete (timeout: 600s)..."
echo "   (This may take a while. Logs will be shown periodically)"

# Wait with periodic log updates
LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --field-selector=status.phase!=Succeeded,status.phase!=Failed -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
TIMEOUT=600
ELAPSED=0
INTERVAL=30

while [ $ELAPSED -lt $TIMEOUT ]; do
    # Check if MPIJob is complete
    JOB_SUCCEEDED=$(kubectl get mpijob matrix-multiplication-mpijob -o jsonpath='{.status.conditions[?(@.type=="Succeeded")].status}' 2>/dev/null || echo "")
    if [ "$JOB_SUCCEEDED" = "True" ]; then
        echo "MPIJob completed successfully!"
        break
    fi
    
    # Check if MPIJob failed
    JOB_FAILED=$(kubectl get mpijob matrix-multiplication-mpijob -o jsonpath='{.status.conditions[?(@.type=="Failed")].status}' 2>/dev/null || echo "")
    if [ "$JOB_FAILED" = "True" ]; then
        echo "MPIJob failed!"
        break
    fi
    
    # Show progress every INTERVAL seconds
    if [ $((ELAPSED % INTERVAL)) -eq 0 ] && [ $ELAPSED -gt 0 ]; then
        echo ""
        echo "[$ELAPSED/$TIMEOUT seconds elapsed] Current status:"
        kubectl get mpijob matrix-multiplication-mpijob -o jsonpath='{.status.conditions[-1].type}: {.status.conditions[-1].message}' 2>/dev/null || echo "Status: Running"
        
        # Refresh launcher pod name (it may have been recreated)
        LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --field-selector=status.phase!=Succeeded,status.phase!=Failed -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
        if [ -n "$LAUNCHER_POD" ]; then
            echo ""
            echo "Recent launcher logs (last 200 lines):"
            kubectl logs "$LAUNCHER_POD" -c launcher --tail=200 2>&1 || echo "Could not get logs"
        fi
        echo ""
    fi
    
    sleep 5
    ELAPSED=$((ELAPSED + 5))
done

# Final check
JOB_SUCCEEDED=$(kubectl get mpijob matrix-multiplication-mpijob -o jsonpath='{.status.conditions[?(@.type=="Succeeded")].status}' 2>/dev/null || echo "")
if [ "$JOB_SUCCEEDED" != "True" ]; then
    echo ""
    echo "=== MPIJob TIMEOUT or FAILED ==="
    echo ""
    echo "MPIJob status:"
    kubectl get mpijob matrix-multiplication-mpijob -o yaml | grep -A 30 "status:" || kubectl get mpijob matrix-multiplication-mpijob
    echo ""
    echo "MPIJob events:"
    kubectl get events --field-selector involvedObject.name=matrix-multiplication-mpijob --sort-by='.lastTimestamp' 2>&1 | tail -20 || true
    
    echo ""
    echo "All pods status (including completed):"
    kubectl get pods -l training.kubeflow.org/job-name=matrix-multiplication-mpijob --show-labels
    echo ""
    echo "All pods with full details:"
    kubectl get pods -l training.kubeflow.org/job-name=matrix-multiplication-mpijob -o wide
    
    echo ""
    echo "=== Launcher pod details ==="
    # Try to find launcher pod (including completed ones)
    LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --field-selector=status.phase!=Failed -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
    if [ -z "$LAUNCHER_POD" ]; then
        # Try to find any launcher pod including failed ones
        LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
    fi
    
    if [ -n "$LAUNCHER_POD" ]; then
        echo "Launcher pod: $LAUNCHER_POD"
        kubectl describe pod "$LAUNCHER_POD" 2>&1 || true
        echo ""
        echo "=== Launcher logs ==="
        kubectl logs "$LAUNCHER_POD" -c launcher --tail=200 2>&1 || echo "Could not get launcher logs"
        # Try to get previous instance logs if pod was restarted
        kubectl logs "$LAUNCHER_POD" -c launcher --previous --tail=200 2>&1 || true
    else
        echo "No launcher pod found in current state!"
        echo "Checking all pods with launcher label (including completed):"
        kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --show-labels 2>&1 || true
        echo ""
        echo "Attempting to get logs by label:"
        kubectl logs -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --tail=200 2>&1 || echo "Could not get launcher logs"
        echo ""
        echo "Checking MPIJob launcher replica status:"
        kubectl get mpijob matrix-multiplication-mpijob -o jsonpath='{.status.replicaStatuses.Launcher}' 2>&1 || true
    fi
    
    echo ""
    echo "=== Worker pods details ==="
    WORKER_PODS=$(kubectl get pods -l role=worker,training.kubeflow.org/job-name=matrix-multiplication-mpijob -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")
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
        kubectl logs -l role=worker,training.kubeflow.org/job-name=matrix-multiplication-mpijob --tail=50 2>&1 | head -100 || echo "Could not get worker logs"
    fi
    
    exit 1
fi

echo ""
echo "7. Printing job logs..."
LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
if [ -n "$LAUNCHER_POD" ]; then
    kubectl logs "$LAUNCHER_POD" -c launcher --tail=200 2>&1 || echo "Could not get launcher logs"
else
    kubectl logs -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob --tail=200 2>&1 || echo "Could not get launcher logs by label"
fi

echo ""
echo "8. Fetching result from PVC..."
RESULT_FILE="./output.txt"
kubectl delete pod matrix-result-reader --ignore-not-found=true
kubectl run matrix-result-reader \
    --image=distribiuted-matrix-multiplication:latest \
    --restart=Never \
    --overrides='{
        "apiVersion": "v1",
        "spec": {
            "volumes": [
                {"name": "matrix-output", "persistentVolumeClaim": {"claimName": "matrix-storage"}}
            ],
            "containers": [
                {
                    "name": "matrix-result-reader",
                    "image": "distribiuted-matrix-multiplication:latest",
                    "imagePullPolicy": "IfNotPresent",
                    "command": ["/bin/sh", "-c", "sleep 3600"],
                    "volumeMounts": [{"name": "matrix-output", "mountPath": "/result-data"}]
                }
            ]
        }
    }' \
    -- /bin/sh -c "sleep 3600"
kubectl wait --for=condition=ready pod/matrix-result-reader --timeout=120s || {
    echo "ERROR: Result reader pod not ready"
    kubectl describe pod matrix-result-reader
    exit 1
}

kubectl cp "default/matrix-result-reader:/result-data/output.txt" "${RESULT_FILE}"
kubectl delete pod matrix-result-reader --ignore-not-found=true

echo "Verifying result..."
if make verify-multiplication MATRIX_A="${MATRIX_A}" MATRIX_B="${MATRIX_B}" RESULT="${RESULT_FILE}"; then
    echo "=== E2E Test PASSED ==="
else
    echo "=== E2E Test FAILED ==="
    exit 1
fi
