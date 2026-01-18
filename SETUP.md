# Przewodnik instalacji Ubuntu - Rozproszone mnożenie macierzy

Ten przewodnik pomoże Ci skonfigurować świeżą instalację Ubuntu do uruchomienia projektu rozproszonego mnożenia macierzy z Kubernetes i MPI.

## Wymagania wstępne

- Świeża instalacja Ubuntu 20.04 lub nowsza
- Połączenie z internetem
- Dostęp sudo/root
- **RAM**: 
  - **Minimum 4GB** - działa z k3s lub zoptymalizowanym Minikube/Kind
  - **Zalecane 8GB+** - dla wygodniejszej pracy z Minikube/Kind
- Minimum 20GB wolnego miejsca na dysku

### Uwaga dotycząca pamięci RAM

**Tak, Kubernetes zadziała na 4GB RAM**, ale z pewnymi ograniczeniami:
- **k3s** - najlepsza opcja dla 4GB (używa ~200-300MB)
- **Minikube** - możliwe, ale wymaga zmniejszenia przydziału pamięci (2GB zamiast 4GB)
- **Kind** - możliwe, ale będzie ciasno (zalecane minimum 4GB tylko dla klastra)

Dla systemu z 4GB RAM zalecamy **k3s** lub **Minikube z ograniczoną pamięcią**.

## Krok 1: Aktualizacja systemu

```bash
sudo apt update && sudo apt upgrade -y
```

## Krok 2: Instalacja podstawowych narzędzi do kompilacji

```bash
sudo apt install -y \
    build-essential \
    curl \
    wget \
    git \
    make \
    ca-certificates \
    gnupg \
    lsb-release \
    software-properties-common \
    apt-transport-https
```

## Krok 3: Instalacja Dockera

### 3.1 Dodanie oficjalnego klucza GPG Dockera
```bash
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg
```

### 3.2 Dodanie repozytorium Dockera
```bash
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
```

### 3.3 Instalacja Docker Engine
```bash
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
```

### 3.4 Dodanie użytkownika do grupy docker (aby uruchamiać docker bez sudo)
```bash
sudo usermod -aG docker $USER
newgrp docker  # Zastosuj zmiany grupy bez wylogowania
```

### 3.5 Weryfikacja instalacji Dockera
```bash
docker --version
docker run hello-world
```

## Krok 4: Instalacja narzędzi Rust

### 4.1 Instalacja Rust przy użyciu rustup
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 4.2 Weryfikacja instalacji Rust
```bash
rustc --version
cargo --version
```

### 4.3 Instalacja wymaganych komponentów Rust
```bash
rustup component add rustfmt clippy
```

## Krok 5: Instalacja Python3 i zależności

```bash
sudo apt install -y python3 python3-pip python3-numpy
```

Weryfikacja:
```bash
python3 --version
python3 -c "import numpy; print(numpy.__version__)"
```

## Krok 6: Instalacja Kubernetes (kubectl)

### 6.1 Pobranie kubectl
```bash
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
```

### 6.2 Instalacja kubectl
```bash
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl
rm kubectl
```

### 6.3 Weryfikacja instalacji kubectl
```bash
kubectl version --client
```

## Krok 7: Konfiguracja klastra Kubernetes

Masz kilka opcji uruchomienia Kubernetes lokalnie. Wybierz jedną:

> **💡 Wskazówka**: Jeśli masz tylko 4GB RAM, wybierz **k3s** (Opcja C) - jest najlżejsza i najlepiej działa na systemach z ograniczoną pamięcią.

### Opcja A: Minikube (Zalecane do rozwoju lokalnego)

#### 7.1 Instalacja Minikube
```bash
curl -LO https://storage.googleapis.com/minikube/releases/latest/minikube-linux-amd64
sudo install minikube-linux-amd64 /usr/local/bin/minikube
rm minikube-linux-amd64
```

#### 7.2 Uruchomienie Minikube

```bash
minikube start --driver=docker --memory=4096 --cpus=2
```

**Uwaga**: Jeśli masz tylko 4GB RAM, użyj 2048MB dla Minikube, aby zostawić pamięć dla systemu i innych procesów.

#### 7.3 Konfiguracja kubectl do użycia Minikube
```bash
minikube kubectl -- get pods -A
```

#### 7.4 Konfiguracja środowiska Docker Minikube (do budowania obrazów)
```bash
eval $(minikube docker-env)
```

#### 7.5 Weryfikacja klastra
```bash
kubectl cluster-info
kubectl get nodes
```

## Krok 8: Instalacja Kubeflow MPI Operator

### 8.1 Instalacja MPI Operator

```bash
kubectl apply --server-side -f https://raw.githubusercontent.com/kubeflow/mpi-operator/v0.7.0/deploy/v2beta1/mpi-operator.yaml
```


### 8.2 Oczekiwanie na gotowość MPI Operator
```bash
kubectl wait --for=condition=available deployment/mpi-operator -n mpi-operator --timeout=300s
```

### 8.3 Weryfikacja instalacji MPI Operator
```bash
kubectl get crd mpijobs.kubeflow.org
kubectl get pods -n mpi-operator
```

## Krok 9: Klonowanie i budowanie projektu

### 9.1 Klonowanie repozytorium (jeśli jeszcze nie zrobione)
```bash
git clone <repository-url>
cd distribiuted-matrix-multiplication
```

### 9.2 Budowanie obrazu Docker

**Jeśli używasz Minikube:**
```bash
# Upewnij się, że środowisko Docker Minikube jest aktywne
eval $(minikube docker-env)
docker build -t distribiuted-matrix-multiplication:latest .
```

**Jeśli używasz Kind:**
```bash
# Budowanie obrazu
docker build -t distribiuted-matrix-multiplication:latest .

# Załadowanie obrazu do klastra Kind
kind load docker-image distribiuted-matrix-multiplication:latest --name matrix-mult
```

**Jeśli używasz k3s:**
```bash
# Budowanie obrazu
docker build -t distribiuted-matrix-multiplication:latest .

# Import obrazu do k3s (k3s używa containerd, więc musimy zaimportować)
docker save distribiuted-matrix-multiplication:latest | sudo k3s ctr images import -
```

**Jeśli używasz pełnego klastra Kubernetes:**
```bash
# Budowanie i wypchnięcie do rejestru (Docker Hub, GCR, itp.)
docker build -t your-registry/distribiuted-matrix-multiplication:latest .
docker push your-registry/distribiuted-matrix-multiplication:latest
# Następnie zaktualizuj mpijob.yaml, aby używał obrazu z rejestru
```

## Krok 10: Weryfikacja instalacji

### 10.1 Sprawdzenie wszystkich komponentów
```bash
# Sprawdzenie Dockera
docker --version
docker ps

# Sprawdzenie Rust
cargo --version

# Sprawdzenie Pythona
python3 --version
python3 -c "import numpy; print('NumPy:', numpy.__version__)"

# Sprawdzenie Kubernetes
kubectl version --client
kubectl cluster-info

# Sprawdzenie MPI Operator
kubectl get crd mpijobs.kubeflow.org
```

### 10.2 Uruchomienie testowej kompilacji
```bash
make build
```

### 10.3 Uruchomienie testu E2E

**Dla systemu z 4GB RAM** - użyj mniejszej liczby workerów:
```bash
make k8s-e2e-test WORKERS=2 MATRIX_SIZE=100
```

**Dla systemu z 8GB+ RAM**:
```bash
make k8s-e2e-test WORKERS=4 MATRIX_SIZE=100
```

## Krok 11: Optymalizacja dla systemów z 4GB RAM

Jeśli masz tylko 4GB RAM, zastosuj następujące optymalizacje:

### 11.1 Wybierz k3s zamiast Minikube/Kind
k3s używa najmniej pamięci (~200-300MB vs ~1-2GB dla Minikube).

### 11.2 Ogranicz zasoby dla MPIJob
Edytuj `k8s/mpijob.yaml` i zmniejsz limity pamięci:
```yaml
resources:
  requests:
    memory: "256Mi"  # Zmniejsz z 512Mi
    cpu: "250m"      # Zmniejsz z 500m
  limits:
    memory: "2Gi"    # Zmniejsz z 8Gi
    cpu: "2"         # Zmniejsz z 8
```

### 11.3 Użyj mniejszej liczby workerów
```bash
# Zamiast 4 workerów, użyj 2
make k8s-e2e-test WORKERS=2 MATRIX_SIZE=100
```

### 11.4 Monitoruj użycie pamięci
```bash
# Sprawdź użycie pamięci systemu
free -h

# Sprawdź użycie pamięci w klastrze
kubectl top nodes
kubectl top pods -A
```

### 11.5 Zamknij niepotrzebne usługi
```bash
# Sprawdź, co zużywa pamięć
ps aux --sort=-%mem | head -20

# Zatrzymaj niepotrzebne usługi (np. jeśli nie używasz)
sudo systemctl stop snapd  # jeśli nie używasz snap
```

## Krok 12: Rozwiązywanie problemów

### Błąd uprawnień Dockera
```bash
# Jeśli otrzymujesz błędy "permission denied" z Dockerem:
sudo usermod -aG docker $USER
newgrp docker
# Lub wyloguj się i zaloguj ponownie
```

### Minikube nie uruchamia się
```bash
# Sprawdź, czy wirtualizacja jest włączona
egrep -c '(vmx|svm)' /proc/cpuinfo
# Powinno zwrócić > 0

# Jeśli używasz sterownika Docker, upewnij się, że Docker działa
sudo systemctl status docker
```

### MPI Operator nie instaluje się

**Problem z kustomize (evalsymlink failure):**
```bash
# Jeśli otrzymujesz błąd "evalsymlink failure", użyj metody z konkretną wersją:

# Najprostsza i najbardziej niezawodna metoda:
kubectl apply --server-side -f https://raw.githubusercontent.com/kubeflow/mpi-operator/v0.7.0/deploy/v2beta1/mpi-operator.yaml

# Alternatywnie: Klonuj i zainstaluj lokalnie
git clone https://github.com/kubeflow/mpi-operator.git
cd mpi-operator
kubectl apply -k config/default
cd ..
rm -rf mpi-operator
```

**Inne problemy:**
```bash
# Sprawdź połączenie z klastrem
kubectl cluster-info

# Sprawdź, czy namespace istnieje
kubectl get namespace mpi-operator

# Jeśli namespace nie istnieje, utwórz go ręcznie
kubectl create namespace mpi-operator

# Sprawdź logi operatora
kubectl logs -n mpi-operator deployment/mpi-operator

# Sprawdź, czy CRD został utworzony
kubectl get crd | grep mpi

# Jeśli CRD nie istnieje, zainstaluj go ręcznie
kubectl apply -f https://raw.githubusercontent.com/kubeflow/mpi-operator/master/deploy/v2beta1/mpi-operator.yaml
```

### Błędy pobierania obrazu
```bash
# Dla Minikube: upewnij się, że używasz demona Docker Minikube
eval $(minikube docker-env)
docker images | grep distribiuted-matrix-multiplication

# Dla Kind: upewnij się, że obraz jest załadowany
kind load docker-image distribiuted-matrix-multiplication:latest --name matrix-mult

# Sprawdź politykę pobierania obrazu w mpijob.yaml (powinno być IfNotPresent)
```

### Problemy z pamięcią (Out of Memory)
```bash
# Jeśli otrzymujesz błędy OOM (Out of Memory):
# 1. Sprawdź użycie pamięci
free -h
kubectl top nodes
kubectl top pods -A

# 2. Zmniejsz liczbę workerów w MPIJob
# Edytuj k8s/mpijob.yaml i zmień WORKER_COUNT na mniejszą wartość (np. 2 zamiast 4)

# 3. Zmniejsz limity pamięci w mpijob.yaml (patrz Krok 11.2)

# 4. Dla Minikube: zmniejsz przydział pamięci
minikube stop
minikube start --driver=docker --memory=2048 --cpus=2

# 5. Dla k3s: sprawdź, czy nie ma zbyt wielu podów
kubectl get pods -A
kubectl delete pod <niepotrzebny-pod>  # usuń niepotrzebne pody

# 6. Sprawdź, czy pody nie są w stanie OOMKilled
kubectl get pods -A | grep OOMKilled
```

## Krok 13: Szybkie polecenia referencyjne

```bash
# Budowanie obrazu Docker
make docker-build

# Sprawdzenie MPI Operator
make k8s-check-mpi-operator

# Instalacja MPI Operator
make k8s-install-mpi-operator

# Uruchomienie testu E2E (dla 4GB RAM użyj WORKERS=2)
make k8s-e2e-test WORKERS=4 MATRIX_SIZE=100

# Wyświetlenie statusu MPIJob
kubectl get mpijobs

# Wyświetlenie logów MPIJob
kubectl logs -l role=launcher,app=distribiuted-matrix-multiplication

# Usunięcie MPIJob
kubectl delete mpijob matrix-multiplication-mpijob
```

## Podsumowanie wymagań systemowych

- **OS**: Ubuntu 20.04 lub nowsza
- **RAM**: 
  - **4GB** - minimum, działa z k3s lub zoptymalizowanym Minikube (2GB dla klastra)
  - **8GB+** - zalecane dla wygodniejszej pracy z Minikube/Kind
- **CPU**: 2+ rdzenie zalecane
- **Dysk**: 20GB+ wolnego miejsca
- **Sieć**: Połączenie z internetem do pobierania

### Zalecenia dla systemów z 4GB RAM:

1. **Użyj k3s** - najlżejsza opcja (~200-300MB RAM)
2. **Lub Minikube z 2GB** - `minikube start --driver=docker --memory=2048 --cpus=2`
3. **Ogranicz liczbę workerów** w MPIJob do 2-3 zamiast 4
4. **Zamknij niepotrzebne aplikacje** przed uruchomieniem klastra
5. **Monitoruj użycie pamięci**: `free -h` i `kubectl top nodes`

## Następne kroki

Po ukończeniu tej konfiguracji:
1. Przejrzyj strukturę projektu
2. Przeczytaj Makefile, aby poznać dostępne polecenia
3. Spróbuj uruchomić test E2E z różną liczbą workerów
4. Eksperymentuj z różnymi rozmiarami macierzy

## Dodatkowe uwagi

- Dla wdrożeń produkcyjnych użyj właściwego rejestru kontenerów
- Rozważ skonfigurowanie limitów zasobów w Kubernetes
- Monitoruj zasoby klastra: `kubectl top nodes`
- Dla potrzeb trwałego magazynu skonfiguruj StorageClasses
