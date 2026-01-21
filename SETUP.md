# Przewodnik instalacji

Ten przewodnik pomoże skonfigurować świeżą instalację Ubuntu do uruchomienia projektu rozproszonego mnożenia macierzy z Kubernetes i MPI. Zostal przetestowany za pomocą AWS EC2 t2.large

## Wymagania wstępne

- Świeża instalacja Ubuntu 24.04 lub nowsza
- Połączenie z internetem
- Dostęp sudo/root
- **RAM**: Zalecane 8GB+
- Minimum 30GB wolnego miejsca na dysku



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
    openmpi-bin \
    libopenmpi-dev \
    pkg-config \
    libc6-dev \
    clang
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

```bash
# Upewnij się, że środowisko Docker Minikube jest aktywne
eval $(minikube docker-env)
docker build -t distribiuted-matrix-multiplication:latest .
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
kubectl get crd mpijobs.kubeflow.org
```

### 10.2 Uruchomienie testowej kompilacji
```bash
make build
```

### 11 Test programu

Przygotowany test E2E demonstruje pełny łańcuch działania aplikacji:

- Budowa obrazu Dockera: buduje obraz aplikacji distribiuted-matrix-multiplication:latest.
- Generowanie danych wejściowych: lokalnie tworzy dwie macierze A i B o rozmiarze MATRIX_SIZE × MATRIX_SIZE.
- Przygotowanie zasobów K8s:
  - aplikuje ConfigMap z ustawieniami MPI,
  - tworzy ConfigMap z plikami macierzy A i B,
  - aplikuje PVC na wynik i czeka aż zostanie zbindowany,
  - aplikuje MPIJob z liczbą workerów WORKER_COUNT.
- Uruchomienie i monitorowanie obliczeń: czeka na zakończenie MPIJob.
- Pobranie wyniku: uruchamia tymczasowy pod czytający wynik, kopiuje output.txt z PVC do lokalnego pliku, a potem usuwa pod.
- Weryfikacja poprawności: lokalnie sprawdza wynik mnożenia macierzy i kończy test statusem „zaliczony/niezaliczony”.


```bash
make k8s-e2e-test WORKER_COUNT=2 MATRIX_SIZE=100
```