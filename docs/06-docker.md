# Part 6: Docker for Rust Applications

This tutorial teaches you how to containerize Rust applications using Docker. You'll learn how to write Dockerfiles, optimize builds, manage contexts with .dockerignore, and orchestrate multiple containers locally with Docker Compose.

## Table of Contents

- [Understanding Docker Fundamentals](#understanding-docker-fundamentals)
- [Writing a Dockerfile for Rust](#writing-a-dockerfile-for-rust)
- [Multi-Stage Builds](#multi-stage-builds)
- [Docker Context and .dockerignore](#docker-context-and-dockerignore)
- [Docker Compose for Local Development](#docker-compose-for-local-development)
- [Building and Running](#building-and-running)
- [Optimization Techniques](#optimization-techniques)

---

## Understanding Docker Fundamentals

### What is Docker?

Docker is a platform for developing, shipping, and running applications in **containers**. A container packages an application with all its dependencies (libraries, runtime, system tools) into a standardized unit.

**Key concepts:**

1. **Image**: A read-only template with instructions for creating a container
   - Like a class in OOP (blueprint)
   - Contains: OS files, application code, dependencies, configuration
   - Immutable once built
   - Stored in registries (Docker Hub, GitHub Container Registry, etc.)

2. **Container**: A running instance of an image
   - Like an object/instance in OOP
   - Has its own filesystem, network, process space
   - Ephemeral by default (data lost when container stops)
   - Can be started, stopped, moved, deleted

3. **Dockerfile**: A text file with instructions to build an image
   - Like a recipe or build script
   - Each instruction creates a layer in the image

4. **Layer**: Each instruction in a Dockerfile creates a layer
   - Layers are cached and reused
   - Makes subsequent builds faster
   - Understanding layers is key to optimization

### Why Containerize?

**Consistency**: "Works on my machine" → "Works everywhere"
- Same environment in development, testing, production
- Eliminates dependency conflicts

**Isolation**: Each container runs independently
- Doesn't interfere with host or other containers
- Security boundary

**Portability**: Run anywhere Docker runs
- Local machine, cloud, on-premises
- Switch providers easily

**Efficiency**: Containers share the host OS kernel
- Lighter than VMs (MBs vs GBs)
- Start in seconds vs minutes

### Docker Architecture

```
┌─────────────────────────────────────┐
│         Docker Client               │
│     (docker build, docker run)      │
└────────────┬────────────────────────┘
             │ REST API
┌────────────▼────────────────────────┐
│       Docker Daemon (dockerd)       │
│  - Manages images, containers       │
│  - Handles builds                   │
└────────┬───────────┬─────────────── ┘
         │           │
    ┌────▼───┐  ┌────▼────┐
    │ Images │  │Containers│
    └────────┘  └─────────┘
```

**How it works:**
1. You write a Dockerfile
2. Run `docker build` → Docker client sends context to daemon
3. Daemon executes Dockerfile instructions → creates image
4. Run `docker run` → Daemon creates container from image
5. Container runs your application

**Documentation**:
- Docker overview: https://docs.docker.com/get-started/overview/
- Architecture: https://docs.docker.com/get-started/docker-concepts/the-basics/what-is-an-image/

---

## Writing a Dockerfile for Rust

A Dockerfile is a text file named `Dockerfile` (no extension) that contains instructions for building an image.

### Basic Dockerfile Instructions

**FROM**: Set the base image
```dockerfile
FROM rust:1.75
```
- Every Dockerfile starts with FROM
- Specifies which image to build upon
- For Rust: `rust:1.75` (includes Rust toolchain)
- For minimal images: `rust:1.75-alpine` (Alpine Linux, ~5MB vs ~120MB)

**WORKDIR**: Set the working directory
```dockerfile
WORKDIR /app
```
- All subsequent commands run from this directory
- Creates the directory if it doesn't exist
- Like `cd /app` that persists

**COPY**: Copy files from host to image
```dockerfile
COPY Cargo.toml Cargo.lock ./
COPY src ./src
```
- Format: `COPY <source> <destination>`
- Source is relative to build context (where you run `docker build`)
- Destination is relative to WORKDIR

**RUN**: Execute a command during build
```dockerfile
RUN cargo build --release
```
- Runs in a shell (`/bin/sh -c` on Linux)
- Each RUN creates a new layer
- Use `&&` to chain commands in one RUN (fewer layers)

**ENV**: Set environment variables
```dockerfile
ENV RUST_LOG=info
```
- Available during build AND runtime
- Format: `ENV KEY=VALUE` or `ENV KEY VALUE`

**EXPOSE**: Document which ports the container listens on
```dockerfile
EXPOSE 8080
```
- Does NOT actually publish the port (use `-p` flag for that)
- Serves as documentation
- Used by tools like Docker Compose

**CMD**: Default command to run when container starts
```dockerfile
CMD ["./target/release/myapp"]
```
- Only one CMD per Dockerfile (last one wins)
- Can be overridden: `docker run myimage /bin/sh`
- Two formats:
  - Exec form (preferred): `CMD ["executable", "arg1"]`
  - Shell form: `CMD executable arg1` (runs in shell)

**ENTRYPOINT**: Configure container as an executable
```dockerfile
ENTRYPOINT ["./target/release/myapp"]
CMD ["--help"]
```
- ENTRYPOINT = the executable
- CMD = default arguments (can be overridden)
- Combined: `./target/release/myapp --help`
- Override CMD: `docker run myimage --version` → `./target/release/myapp --version`

### Simple Rust Dockerfile Example

```dockerfile
# Start from official Rust image
FROM rust:1.75

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Build the application
RUN cargo build --release

# Run the application
CMD ["./target/release/whitewater"]
```

**Build it:**
```bash
docker build -t whitewater:v1 .
```

**Run it:**
```bash
docker run whitewater:v1
```

**Problems with this approach:**
1. **Huge image**: ~1.5GB (includes entire Rust toolchain)
2. **Slow builds**: Recompiles dependencies every time source changes
3. **Security**: Build tools in production image (unnecessary attack surface)

**Solution**: Multi-stage builds (next section)

---

## Multi-Stage Builds

Multi-stage builds solve the problems above by using multiple FROM statements. Each FROM starts a new build stage.

### How Multi-Stage Builds Work

```dockerfile
# Stage 1: Builder
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM alpine:3.18
COPY --from=builder /app/target/release/whitewater /app/whitewater
CMD ["/app/whitewater"]
```

**What happens:**
1. Stage 1 (builder): Compile the application using full Rust image
2. Stage 2 (runtime): Create minimal image, copy only the compiled binary
3. Final image: Only contains stage 2 (~20MB vs ~1.5GB)

**Benefits:**
- Small final image (faster deployment)
- No build tools in production (better security)
- Separation of concerns (build vs runtime)

### Optimizing for Cargo Build Cache

Cargo downloads and compiles dependencies every time you change source code. We can fix this by copying Cargo files first and building dependencies separately.

```dockerfile
# Stage 1: Builder
FROM rust:1.75-alpine AS builder

# Install build dependencies (musl for static linking on Alpine)
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy only Cargo files first
COPY Cargo.toml Cargo.lock ./

# Create dummy source to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer is cached until Cargo.toml changes)
RUN cargo build --release

# Remove dummy source and target binary
RUN rm -rf src target/release/whitewater*

# Copy real source code
COPY src ./src

# Build the actual application
RUN cargo build --release

# Strip debug symbols to reduce size
RUN strip target/release/whitewater

# Stage 2: Runtime
FROM alpine:3.18

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

# Create non-root user for security
RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/whitewater /app/whitewater

# Switch to non-root user
USER appuser

# Document the port
EXPOSE 8080

# Run the application
CMD ["/app/whitewater"]
```

**How caching works:**
1. First build: Compiles dependencies (slow, ~5 min)
2. Change source code, rebuild: Dependencies cached, only app recompiled (fast, ~30s)
3. Change Cargo.toml, rebuild: Full rebuild (slow)

### Understanding FROM Variations

**rust:1.75** (Debian-based, ~1.2GB)
- Full Linux system
- All build tools included
- Easy to use, but large

**rust:1.75-alpine** (Alpine Linux, ~300MB)
- Minimal Linux distribution
- Uses musl libc instead of glibc
- Smaller but requires understanding differences

**rust:1.75-slim** (Debian-based, ~600MB)
- Debian without unnecessary packages
- Middle ground between full and Alpine

**alpine:3.18** (Runtime, ~5MB)
- Minimal OS for final image
- Only OS files, no development tools
- Perfect for runtime images

### USER Instruction (Security)

Running containers as root is a security risk. Create a non-root user:

```dockerfile
# In runtime stage
RUN addgroup -g 1000 myapp && \
    adduser -D -u 1000 -G myapp myapp

USER myapp
```

**Why:**
- If container is compromised, attacker only has user permissions
- Best practice for production
- Some orchestrators enforce non-root

**Documentation**: https://docs.docker.com/build/building/multi-stage/

---

## Docker Context and .dockerignore

### What is the Build Context?

When you run `docker build .`, the `.` is the **build context** - all files in that directory are sent to the Docker daemon.

```bash
docker build -t myapp .
             └─ build context directory
```

**What gets sent:**
- All files and subdirectories in the build context
- Even files you don't COPY in the Dockerfile!

**Problem**: If you have large directories (like `target/`, `.git/`), build context can be gigabytes, making builds slow.

### .dockerignore File

`.dockerignore` works like `.gitignore` - it tells Docker which files to exclude from the build context.

**Location**: Same directory as Dockerfile

**Syntax**: Same as `.gitignore`
- Patterns are relative to context directory
- `*` matches any sequence except `/`
- `**` matches any sequence including `/`
- `!` negates a pattern

**Example .dockerignore for Rust:**

```dockerignore
# Build artifacts
target/
**/*.rs.bk

# Git
.git/
.gitignore

# Docker
Dockerfile
.dockerignore
docker-compose.yml

# IDE
.vscode/
.idea/
*.swp

# Documentation
*.md
docs/

# OS
.DS_Store
Thumbs.db

# Secrets (IMPORTANT!)
.env
*.pem
*.key
```

**Why each exclusion:**

**target/**: Cargo build output (~1GB)
- We build inside Docker, don't need host's build artifacts
- Huge directory that slows context upload

**.git/**: Git repository data (~100MB+)
- Version control history not needed in image
- Security: might contain sensitive commit messages

**Dockerfile, .dockerignore**: Build instructions
- Used to build image, but not needed inside image

**IDE files (.vscode/, .idea/)**: Editor configuration
- Personal preferences, not needed in container

**Documentation (*.md, docs/)**: README, docs
- Not needed at runtime (unless you're serving docs)

**.env**: Environment variables file
- **CRITICAL**: Often contains secrets
- Must be excluded to prevent leaking credentials

**Why .dockerignore matters:**
1. **Speed**: Smaller context = faster builds
2. **Security**: Prevents accidentally copying secrets
3. **Determinism**: Ensures only necessary files affect builds
4. **Cache efficiency**: Fewer files = better cache hit rate

**Test your .dockerignore:**
```bash
# Create a context tarball (what Docker sees)
tar -czf context.tar.gz --exclude-from .dockerignore .

# List what's in it
tar -tzf context.tar.gz | less
```

**Documentation**: https://docs.docker.com/build/building/context/

---

## Docker Compose for Local Development

Docker Compose is a tool for defining and running multi-container applications.

### Why Use Docker Compose?

**Without Compose** (manual Docker commands):
```bash
docker network create raft-net
docker run -d --name raft-0 --network raft-net -p 8080:8080 -e NODE_ID=0 whitewater
docker run -d --name raft-1 --network raft-net -p 8081:8080 -e NODE_ID=1 whitewater
docker run -d --name raft-2 --network raft-net -p 8082:8080 -e NODE_ID=2 whitewater
```
- Tedious, error-prone
- Hard to remember all the flags
- No easy way to share with team

**With Compose** (docker-compose.yml):
```bash
docker-compose up
```
- Start all containers with one command
- Configuration in version-controlled file
- Easy to share and reproduce

### docker-compose.yml Structure

Docker Compose uses YAML files to define services, networks, and volumes.

**Basic example:**

```yaml
version: '3.8'

services:
  raft-node-0:
    build: .
    ports:
      - "8080:8080"
    environment:
      - NODE_ID=0
    networks:
      - raft-network

  raft-node-1:
    build: .
    ports:
      - "8081:8080"
    environment:
      - NODE_ID=1
    networks:
      - raft-network

networks:
  raft-network:
    driver: bridge
```

### Key Compose Concepts

**version**: Compose file format version
- `3.8` is widely supported
- Different versions have different features
- See: https://docs.docker.com/compose/compose-file/compose-versioning/

**services**: Container definitions
- Each service becomes a container
- Can scale: `docker-compose up --scale raft-node=5`

**build**: How to build the image
```yaml
build:
  context: .              # Where to find Dockerfile
  dockerfile: Dockerfile  # Which Dockerfile to use
```

**image**: Tag the built image or pull from registry
```yaml
image: whitewater:latest  # Tag built image
# OR
image: nginx:alpine       # Pull from Docker Hub
```

**container_name**: Explicit container name
```yaml
container_name: raft-node-0  # Instead of projectname_service_1
```
- Makes names predictable
- Can't scale this service beyond 1 replica

**ports**: Map host ports to container ports
```yaml
ports:
  - "8080:80"     # host:container
  - "9000:9000"
```
- Format: `"HOST_PORT:CONTAINER_PORT"`
- Host port must be unique across all services

**environment**: Set environment variables
```yaml
environment:
  - NODE_ID=0
  - RUST_LOG=debug
  - PEERS=raft-1:9000,raft-2:9000
```
- Available inside container via `std::env::var()`
- Can also use `env_file:` to load from a file

**networks**: Which networks to join
```yaml
networks:
  - raft-network
```
- Services on same network can communicate by service name
- DNS: `raft-node-1` resolves to that container's IP

**volumes**: Persistent storage
```yaml
volumes:
  - ./data:/app/data        # bind mount: host:container
  - raft-logs:/app/logs     # named volume
```
- Bind mount: direct access to host filesystem
- Named volume: managed by Docker

**depends_on**: Start order
```yaml
depends_on:
  - database
```
- Waits for container to START (not be READY)
- Doesn't wait for health checks

**restart**: Restart policy
```yaml
restart: on-failure  # Restart if exits with error
```
- `no`: Never restart
- `always`: Always restart
- `on-failure`: Restart on error
- `unless-stopped`: Always unless manually stopped

### Complete Example for 3-Node Raft Cluster

Create a file named `docker-compose.yml`:

```yaml
version: '3.8'

services:
  raft-node-0:
    build:
      context: .
      dockerfile: Dockerfile
    image: whitewater-raft:latest
    container_name: raft-node-0
    networks:
      - raft-network
    ports:
      - "8080:8080"
      - "9000:9000"
    environment:
      - NODE_ID=0
      - PEERS=raft-node-1:9000,raft-node-2:9000
      - RUST_LOG=debug
    volumes:
      - ./data/node-0:/app/data
    restart: on-failure

  raft-node-1:
    build:
      context: .
    image: whitewater-raft:latest
    container_name: raft-node-1
    networks:
      - raft-network
    ports:
      - "8081:8080"
      - "9001:9000"
    environment:
      - NODE_ID=1
      - PEERS=raft-node-0:9000,raft-node-2:9000
      - RUST_LOG=debug
    volumes:
      - ./data/node-1:/app/data
    restart: on-failure

  raft-node-2:
    build:
      context: .
    image: whitewater-raft:latest
    container_name: raft-node-2
    networks:
      - raft-network
    ports:
      - "8082:8080"
      - "9002:9000"
    environment:
      - NODE_ID=2
      - PEERS=raft-node-0:9000,raft-node-1:9000
      - RUST_LOG=debug
    volumes:
      - ./data/node-2:/app/data
    restart: on-failure

networks:
  raft-network:
    driver: bridge
```

**How networking works:**
- All containers join `raft-network`
- `raft-node-0` can reach `raft-node-1` at `raft-node-1:9000`
- Docker's built-in DNS resolves service names to IPs

**How ports work:**
- Each node listens on port 8080 internally
- Mapped to different host ports: 8080, 8081, 8082
- Your Mac can access them at `localhost:8080`, `localhost:8081`, `localhost:8082`

**Documentation**: https://docs.docker.com/compose/compose-file/

---

## Building and Running

### Docker Commands

**Build an image:**
```bash
docker build -t whitewater:v1 .
```
- `-t`: Tag (name:version)
- `.`: Build context directory

**Build with specific Dockerfile:**
```bash
docker build -f Dockerfile.dev -t whitewater:dev .
```

**Build without cache:**
```bash
docker build --no-cache -t whitewater:v1 .
```

**Run a container:**
```bash
docker run whitewater:v1
```

**Run with port mapping:**
```bash
docker run -p 8080:8080 whitewater:v1
```

**Run detached (background):**
```bash
docker run -d -p 8080:8080 whitewater:v1
```

**Run with environment variables:**
```bash
docker run -e NODE_ID=0 -e RUST_LOG=debug whitewater:v1
```

**Run with volume:**
```bash
docker run -v $(pwd)/data:/app/data whitewater:v1
```

**View running containers:**
```bash
docker ps
```

**View logs:**
```bash
docker logs <container-id>
docker logs -f <container-id>  # Follow logs
```

**Execute command in container:**
```bash
docker exec -it <container-id> sh
```

**Stop container:**
```bash
docker stop <container-id>
```

**Remove container:**
```bash
docker rm <container-id>
```

**Remove image:**
```bash
docker rmi whitewater:v1
```

### Docker Compose Commands

**Start all services:**
```bash
docker-compose up
```

**Start in background:**
```bash
docker-compose up -d
```

**Rebuild and start:**
```bash
docker-compose up --build
```

**View logs:**
```bash
docker-compose logs
docker-compose logs -f                # All services
docker-compose logs -f raft-node-0    # One service
```

**List running services:**
```bash
docker-compose ps
```

**Stop services:**
```bash
docker-compose stop
```

**Stop and remove containers:**
```bash
docker-compose down
```

**Stop and remove volumes:**
```bash
docker-compose down -v
```

**Execute command in service:**
```bash
docker-compose exec raft-node-0 sh
```

**Scale a service:**
```bash
docker-compose up --scale raft-node=5
```
(Only works if you don't set `container_name`)

---

## Optimization Techniques

### 1. Layer Caching

Each Dockerfile instruction creates a layer. Layers are cached.

**Bad** (cache invalidated on every source change):
```dockerfile
COPY . .
RUN cargo build --release
```

**Good** (dependencies cached separately):
```dockerfile
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src
COPY src ./src
RUN cargo build --release
```

### 2. Minimize Image Size

**Use Alpine base images:**
```dockerfile
FROM rust:1.75-alpine AS builder
FROM alpine:3.18
```

**Strip binaries:**
```dockerfile
RUN strip target/release/whitewater
```

**Use cargo with size optimizations** (in Cargo.toml):
```toml
[profile.release]
strip = true
opt-level = "z"  # Optimize for size
lto = true       # Link-time optimization
codegen-units = 1
```

### 3. Multi-Stage Build Pattern

Always use multi-stage builds for compiled languages:
- Heavy builder stage (has compilers, build tools)
- Light runtime stage (only runtime dependencies + binary)

### 4. .dockerignore

Exclude everything unnecessary from build context.

### 5. BuildKit

Enable Docker BuildKit for better performance:
```bash
export DOCKER_BUILDKIT=1
docker build -t whitewater:v1 .
```

Or in docker-compose.yml:
```yaml
services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    environment:
      DOCKER_BUILDKIT: 1
```

---

## Next Steps

Now that you understand Docker fundamentals, you're ready to:
1. Write your own Dockerfile for the Raft application
2. Create a .dockerignore file to optimize builds
3. Set up docker-compose.yml for local multi-node testing
4. Move on to Kubernetes for production orchestration

**Continue to**: [Part 7: Kubernetes Concepts](07-kubernetes-concepts.md)

**Documentation**:
- Docker documentation: https://docs.docker.com/
- Dockerfile reference: https://docs.docker.com/engine/reference/builder/
- Docker Compose reference: https://docs.docker.com/compose/compose-file/
- Best practices: https://docs.docker.com/develop/dev-best-practices/
