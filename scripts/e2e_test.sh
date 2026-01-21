#!/bin/bash
set -e

export WORKER_COUNT=${1:-4}
export MATRIX_SIZE=${2:-100}

echo "1. Sprzątanie poprzednich zasobów..."
kubectl delete mpijob matrix-multiplication-mpijob --ignore-not-found=true
kubectl delete pods -l training.kubeflow.org/job-name=matrix-multiplication-mpijob --ignore-not-found=true
kubectl delete pod matrix-result-reader --ignore-not-found=true
kubectl delete configmap matrix-data-config --ignore-not-found=true
echo ""
echo ""

echo "=== Test E2E: $WORKER_COUNT workerów, macierze ${MATRIX_SIZE}x${MATRIX_SIZE} ==="
echo ""
echo ""
echo "2. Sprawdzanie dostępności obrazu Dockera..."


docker build -t distribiuted-matrix-multiplication:latest . || {
    echo "BŁĄD: Nie udało się zbudować obrazu Dockera"
    exit 1
}

echo ""
echo ""
echo "3. Generowanie macierzy lokalnie..."
TMP_DIR=$(mktemp -d)
MATRIX_A="${TMP_DIR}/matrix_a.txt"
MATRIX_B="${TMP_DIR}/matrix_b.txt"
trap 'rm -rf "${TMP_DIR}"; kubectl delete configmap matrix-data-config --ignore-not-found=true >/dev/null 2>&1 || true' EXIT

echo "  Generowanie macierzy lokalnie..."
make generate-matrices SIZE="${MATRIX_SIZE}" OUTPUT_A="${MATRIX_A}" OUTPUT_B="${MATRIX_B}"

echo ""
echo ""
echo "4. Tworzenie ConfigMap i MPIJob..."
# Apply MPI env ConfigMap
kubectl apply -f k8s/mpi-env-configmap.yaml

# Recreate matrix data ConfigMap from local files
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
    echo "Oczekiwanie na powiązanie PVC... ($i/30)"
    sleep 2
done
if [ "$PVC_PHASE" != "Bound" ]; then
    echo "BŁĄD: PVC matrix-storage nie jest powiązany"
    kubectl get pvc matrix-storage -o wide || true
    exit 1
fi

TMP_FILE=$(mktemp)
sed "s/\${WORKER_COUNT}/${WORKER_COUNT}/g" k8s/mpijob.yaml > "$TMP_FILE"

kubectl apply -f "$TMP_FILE"
rm "$TMP_FILE"

echo ""
echo ""
echo "5. Oczekiwanie na zakończenie MPIJob (limit czasu: 600s)..."
TIMEOUT=600
ELAPSED=0
INTERVAL=30
LOG_PID=""

while [ $ELAPSED -lt $TIMEOUT ]; do
    JOB_SUCCEEDED=$(kubectl get mpijob matrix-multiplication-mpijob \
        -o jsonpath='{.status.conditions[?(@.type=="Succeeded")].status}' \
        2>/dev/null || true)

    if [ "$JOB_SUCCEEDED" = "True" ]; then
        echo "MPIJob zakończony pomyślnie!"
        break
    fi

    if [ -z "$LOG_PID" ]; then
        LAUNCHER_POD=$(kubectl get pods -l role=launcher,training.kubeflow.org/job-name=matrix-multiplication-mpijob \
            --field-selector=status.phase!=Failed,status.phase!=Succeeded \
            -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
        if [ -n "$LAUNCHER_POD" ]; then
            echo "Uruchamiam logi w czasie rzeczywistym z: ${LAUNCHER_POD}"
            kubectl logs "$LAUNCHER_POD" -c launcher -f &
            LOG_PID=$!
        fi
    fi

    sleep 5
    ELAPSED=$((ELAPSED + 5))
done

JOB_SUCCEEDED=$(kubectl get mpijob matrix-multiplication-mpijob \
    -o jsonpath='{.status.conditions[?(@.type=="Succeeded")].status}' \
    2>/dev/null || true)
if [ -n "$LOG_PID" ]; then
    kill "$LOG_PID" >/dev/null 2>&1 || true
fi
if [ "$JOB_SUCCEEDED" != "True" ]; then
    echo "MPIJob nie zakończył się pomyślnie w ciągu ${TIMEOUT}s"
    exit 1
fi

echo ""
echo ""
echo "6. Pobieranie wyniku z PVC..."
RESULT_FILE="./output.txt"
kubectl delete pod matrix-result-reader --ignore-not-found=true
kubectl apply -f k8s/result-reader-pod.yaml
kubectl wait --for=condition=ready pod/matrix-result-reader --timeout=120s || {
    echo "BŁĄD: Pod czytnika wyniku nie jest gotowy"
    kubectl describe pod matrix-result-reader
    exit 1
}

kubectl cp "default/matrix-result-reader:/result-data/output.txt" "${RESULT_FILE}"
kubectl delete pod matrix-result-reader --ignore-not-found=true

echo ""
echo ""
echo "7. Weryfikacja wyniku..."
if make verify-multiplication MATRIX_A="${MATRIX_A}" MATRIX_B="${MATRIX_B}" RESULT="${RESULT_FILE}"; then
    echo "=== Test E2E ZALICZONY ==="
else
    echo "=== Test E2E NIEZALICZONY ==="
    exit 1
fi
