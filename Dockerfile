FROM rust:1.78-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libopenmpi-dev \
    openmpi-bin \
    clang \
    libclang-dev \
    make \
    autoconf \
    automake \
    libtool \
    && rm -rf /var/lib/apt/lists/*

# Set environment variables for MPI compilation
ENV PKG_CONFIG_PATH=/usr/lib/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig
ENV LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu/openmpi/lib

# Copy Cargo files first (for better caching)
COPY Cargo.toml ./
# Copy Cargo.lock if it exists
COPY Cargo.lock* ./

# Copy source code
COPY src ./src

# Build the application with verbose output for debugging
RUN set -e && \
    echo "Building application..." && \
    cargo build --release --verbose && \
    echo "Build completed successfully" && \
    ls -lh target/release/distribiuted-matrix-multiplication

# Verify build succeeded
RUN test -f target/release/distribiuted-matrix-multiplication || (echo "ERROR: Build failed - binary not found!" && ls -la target/release/ && exit 1)

FROM debian:bookworm-slim

WORKDIR /app

# Install OpenMPI, Python, and required dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openmpi-bin \
    libopenmpi-dev \
    openssh-client \
    python3 \
    python3-numpy \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/distribiuted-matrix-multiplication /app/distribiuted-matrix-multiplication

# Verify the binary exists and is executable
RUN ls -lh /app/distribiuted-matrix-multiplication && \
    file /app/distribiuted-matrix-multiplication && \
    chmod +x /app/distribiuted-matrix-multiplication

# Create SSH directory for MPI
RUN mkdir -p /home/appuser/.ssh && \
    chmod 700 /home/appuser/.ssh

RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app /home/appuser

USER appuser

# Set MPI environment variables
ENV OMPI_ALLOW_RUN_AS_ROOT=1
ENV OMPI_ALLOW_RUN_AS_ROOT_CONFIRM=1

ENTRYPOINT ["/app/distribiuted-matrix-multiplication"]

