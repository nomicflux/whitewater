# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Whitewater is a simple REST API built with Rust using Axum framework. It provides CRUD operations for user management and is designed to be deployed on Kubernetes using Kind for local development.

## Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Run the application locally (listens on port 8090)
cargo run

# Build for release
cargo build --release

# Install the binary
cargo install --path .
```

### Testing and Quality
```bash
# Run tests
cargo test

# Check for compilation errors without building
cargo check

# Format code
cargo fmt

# Run clippy for linting
cargo clippy
```

### Docker Operations
```bash
# Build Docker image
docker build -t whitewater:1 .

# Run in Docker
docker run -p 8090:8090 whitewater:1
```

### Kubernetes Local Development
```bash
# Create Kind cluster with port mapping
kind create cluster --config kind-cluster.yaml

# Load Docker image into Kind
kind load docker-image whitewater:1 --name whitewater-cluster

# Apply ingress controller
kubectl apply -f ingress-controller.yaml

# Deploy application
kubectl apply -f deploy.yaml

# Full reset and redeploy
./reset_kind.sh
```

### API Testing
```bash
# Create a user
curl -X POST http://localhost:8090/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe", "email": "john@example.com"}'

# List all users
curl http://localhost:8090/users

# Get specific user
curl http://localhost:8090/users/1
```

## Architecture

### Core Components
- **main.rs**: Single-file application containing the entire REST API
- **AppState**: Shared state using Arc<Mutex<HashMap>> for thread-safe user storage
- **User struct**: Data model with id, name, and email fields
- **API endpoints**: POST /users, GET /users, GET /users/:id

### Kubernetes Architecture
- **Deployment**: Single replica deployment exposing port 8090
- **Service**: ClusterIP service for internal communication
- **Ingress**: NGINX ingress controller routing traffic from whitewater.local
- **Kind cluster**: Single control-plane node with port mapping to host port 8090

### Development Workflow
1. Code changes are made to src/main.rs
2. Build and test locally with `cargo run`
3. Build Docker image with `docker build -t whitewater:1 .`
4. Deploy to Kind cluster using `./reset_kind.sh` for full refresh
5. Access via http://localhost:8090 or http://whitewater.local (if ingress configured)

### Key Files
- `Cargo.toml`: Rust dependencies (axum, serde, tokio)
- `deploy.yaml`: Kubernetes manifests for deployment, service, and ingress
- `kind-cluster.yaml`: Kind cluster configuration with port forwarding
- `reset_kind.sh`: Script to recreate cluster and redeploy application
- `Dockerfile`: Multi-stage build using rust:1.90 base image