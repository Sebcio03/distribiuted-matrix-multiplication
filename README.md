# Distributed Matrix Multiplication

## Monitoring (CLI)

Quick CPU/RAM view for all MPIJob pods (namespace `default`).

Enable metrics-server in Minikube (one-time):

```
minikube addons enable metrics-server
```

List the job pods (labels defined in `k8s/mpijob.yaml`):

```
kubectl get pods -n default -l app=distribiuted-matrix-multiplication -o wide
```

Show live CPU/RAM usage for all pods:

```
kubectl top pods -n default -l app=distribiuted-matrix-multiplication
```

Auto-refresh every 2 seconds:

```
watch -n 2 kubectl top pods -n default -l app=distribiuted-matrix-multiplication
```

## Monitoring (K9s)

K9s gives an htop-like live view of pods and container metrics.

Install (macOS):

```
brew install k9s
```

Run:

```
k9s -n default
```

Tip: press `/` in K9s and filter by `app=distribiuted-matrix-multiplication`.