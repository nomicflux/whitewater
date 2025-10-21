# Part 0: Prerequisites & Setup (OSX Sequoia)

This guide will walk you through setting up all the necessary tools on macOS Sequoia for building a distributed Raft implementation. We'll install Rust, Docker Desktop with Kubernetes support, and essential command-line tools.

## Why These Tools?

- **Rust**: Systems programming language with memory safety and zero-cost abstractions
- **Docker**: Containerization for consistent deployment across environments
- **Kubernetes**: Orchestration platform for managing distributed applications
- **kubectl**: Command-line tool for interacting with Kubernetes clusters
- **k9s** (optional): Terminal UI for managing Kubernetes clusters

---

## Step 1: Install Rust

Rust is installed via `rustup`, the official Rust toolchain installer.

```bash
# Install rustup (follows official instructions from https://rustup.rs/)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts (default installation is recommended)
# This will install:
# - rustc (Rust compiler)
# - cargo (Rust package manager and build tool)
# - rustup (toolchain manager)

# Reload your shell configuration
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should show: rustc 1.x.x
cargo --version  # Should show: cargo 1.x.x
```

**What you get:**
- `rustc`: The Rust compiler that turns `.rs` files into executables
- `cargo`: Package manager, build tool, test runner, and documentation generator
- `rustup`: Manages Rust versions and toolchains (stable, nightly, beta)

**Documentation**: https://doc.rust-lang.org/book/ch01-01-installation.html

---

## Step 2: Install Docker Desktop for Mac

Docker Desktop includes both Docker Engine and an optional single-node Kubernetes cluster, perfect for local development.

```bash
# Option 1: Download directly
# Visit: https://www.docker.com/products/docker-desktop/
# Download for your architecture (Apple Silicon or Intel)

# Option 2: Use Homebrew
brew install --cask docker

# After installation, launch Docker Desktop from Applications
# You'll see the Docker icon in your menu bar when it's running
```

**Why Docker Desktop?**
- Includes Kubernetes support (we'll enable this next)
- VM-based isolation for containers on macOS (uses Apple's Hypervisor framework)
- GUI for managing containers, images, volumes, and settings
- Automatic resource management and updates
- File sharing between macOS and containers

**Initial Configuration:**
1. Open Docker Desktop
2. Go to Preferences → Resources
3. Allocate appropriate resources:
   - **CPUs**: 4 minimum (6-8 recommended for Raft cluster)
   - **Memory**: 6GB minimum (8GB+ recommended)
   - **Swap**: 1GB
   - **Disk image size**: 60GB+ (default is usually fine)
4. Click "Apply & Restart"

**Why these resources?**
- Each Raft node will be a separate container
- Kubernetes itself needs ~2GB RAM
- 3-node Raft cluster + Kubernetes = ~4-5GB RAM usage
- Extra headroom prevents swapping and ensures smooth operation

**Verify Docker is running:**
```bash
# Check Docker version
docker --version
# Should show: Docker version 4.36.x or later

# Check Docker is responding
docker ps
# Should show an empty list (no containers running yet)

# Check Docker info
docker info
# Shows detailed system information
```

**Documentation**:
- Installation: https://docs.docker.com/desktop/install/mac-install/
- Settings: https://docs.docker.com/desktop/settings/mac/
- Architecture: https://docs.docker.com/desktop/vm-vdi/

---

## Step 3: Enable Kubernetes in Docker Desktop

Docker Desktop includes a single-node Kubernetes cluster that runs inside the Docker VM. This is perfect for local development and testing.

**Steps:**
1. Open Docker Desktop
2. Click the gear icon (Settings)
3. Navigate to **Kubernetes** in the left sidebar
4. Check the box: **"Enable Kubernetes"**
5. Click **"Apply & Restart"**

**What happens during setup:**
- Docker Desktop downloads Kubernetes system images (~500MB)
- Creates a single-node cluster with all Kubernetes components
- Configures `kubectl` to connect to this cluster automatically
- Starts all Kubernetes system pods (scheduler, controller-manager, etc.)
- Takes 2-5 minutes on first enable (depending on network speed)

**Why Docker Desktop's Kubernetes?**

**Pros:**
- Zero configuration required (works out of the box)
- Shares Docker images directly - no separate container registry needed
- Easy reset: disable/enable Kubernetes to start completely fresh
- Integrated with Docker Desktop GUI
- Sufficient for development and testing distributed systems
- Same API as production Kubernetes clusters

**Cons (for awareness):**
- Single-node only (can't test real multi-node scenarios)
- Different from production (managed K8s services like EKS, GKE, AKS)
- Consumes host resources even when not actively used

**Verify Kubernetes is running:**
```bash
# Check kubectl is installed and configured
kubectl version --client
# Should show: Client Version: v1.31.x or similar

# Check cluster info
kubectl cluster-info
# Should show: Kubernetes control plane is running at https://kubernetes.docker.internal:6443

# Check nodes (should show one node)
kubectl get nodes
# Output:
# NAME             STATUS   ROLES           AGE   VERSION
# docker-desktop   Ready    control-plane   1m    v1.31.x

# Check all system pods are running
kubectl get pods -n kube-system
# Should show ~10 pods, all with STATUS Running or Completed

# Check available namespaces
kubectl get namespaces
# Shows: default, kube-system, kube-public, kube-node-lease
```

**Troubleshooting:**

If Kubernetes fails to start:
1. Docker Desktop → Settings → Kubernetes → "Reset Kubernetes Cluster"
2. Wait 5 minutes for fresh installation
3. If still failing, increase CPU/Memory in Docker Desktop settings
4. Check Docker Desktop logs: Settings → Troubleshoot → View logs

**Alternatives** (not covered in this tutorial, but good to know):
- **Minikube**: https://minikube.sigs.k8s.io/ - More configurable, multi-node support
- **kind** (Kubernetes in Docker): https://kind.sigs.k8s.io/ - Multiple clusters, CI/CD friendly
- **k3s**: https://k3s.io/ - Lightweight Kubernetes, good for edge/IoT

**Documentation**: https://docs.docker.com/desktop/kubernetes/

---

## Step 4: Install and Configure kubectl

`kubectl` (pronounced "kube-control" or "kube-cuttle") is installed automatically by Docker Desktop, but let's verify and configure it properly.

```bash
# Check if kubectl is installed
kubectl version --client --output=yaml

# If not installed (rare), install via Homebrew:
brew install kubectl
```

**Configure shell completion** (highly recommended - saves typing):

For **zsh** (default on macOS):
```bash
# Add to ~/.zshrc
echo 'source <(kubectl completion zsh)' >> ~/.zshrc

# Add alias for convenience
echo 'alias k=kubectl' >> ~/.zshrc
echo 'complete -F __start_kubectl k' >> ~/.zshrc

# Reload
source ~/.zshrc
```

For **bash**:
```bash
# Add to ~/.bash_profile
echo 'source <(kubectl completion bash)' >> ~/.bash_profile
echo 'alias k=kubectl' >> ~/.bash_profile
echo 'complete -F __start_kubectl k' >> ~/.bash_profile

# Reload
source ~/.bash_profile
```

**Now you can:**
```bash
# Use tab completion
kubectl get po<TAB>  # Completes to "pods"
kubectl get pods -n ku<TAB>  # Completes to "-n kube-system"

# Use short alias
k get pods  # Same as kubectl get pods
```

**What is kubectl?**

`kubectl` is the command-line interface for Kubernetes. It communicates with the Kubernetes API server to:
- Deploy applications (create pods, services, deployments)
- Inspect and debug resources
- View logs from containers
- Execute commands inside containers
- Manage cluster configuration

**Architecture:**
```
You → kubectl → Kubernetes API Server → etcd (cluster state)
                      ↓
        Controllers, Scheduler, kubelet
                      ↓
                Actual pods/containers
```

**Essential kubectl commands** (we'll use these throughout the tutorial):

```bash
# Viewing resources
kubectl get pods                    # List all pods in current namespace
kubectl get pods -A                 # List all pods in all namespaces
kubectl get services                # List services
kubectl get nodes                   # List cluster nodes
kubectl get all                     # List most common resources

# Detailed information
kubectl describe pod <pod-name>     # Detailed info about a pod
kubectl describe node docker-desktop  # Node details and resource usage

# Logs and debugging
kubectl logs <pod-name>             # View pod logs
kubectl logs -f <pod-name>          # Follow logs (like tail -f)
kubectl logs <pod-name> -c <container>  # Logs from specific container

# Interactive access
kubectl exec -it <pod-name> -- sh   # Open shell in pod
kubectl exec <pod-name> -- ls /     # Run command in pod

# Resource management
kubectl apply -f <file.yaml>        # Create/update resources from file
kubectl apply -f <directory>        # Apply all yaml files in directory
kubectl delete -f <file.yaml>       # Delete resources defined in file
kubectl delete pod <pod-name>       # Delete specific pod

# Port forwarding
kubectl port-forward <pod-name> 8080:80  # Forward local:8080 → pod:80
kubectl port-forward service/<svc> 8080:80  # Forward to service

# Context and configuration
kubectl config get-contexts         # List available clusters
kubectl config use-context <name>   # Switch cluster
kubectl config current-context      # Show current cluster
```

**Documentation**:
- Overview: https://kubernetes.io/docs/reference/kubectl/
- Cheat sheet: https://kubernetes.io/docs/reference/kubectl/cheatsheet/
- Command reference: https://kubernetes.io/docs/reference/kubectl/kubectl/

---

## Step 5: Install Helm (Optional)

Helm is the package manager for Kubernetes. While we won't use it in this tutorial (we'll write manifests manually to understand them), it's valuable for real-world use.

```bash
# Install via Homebrew
brew install helm

# Verify
helm version
# Should show: version.BuildInfo{Version:"v3.x.x", ...}

# See available public charts
helm search hub wordpress
helm search hub postgresql
```

**What is Helm?**
- Think of it as "apt/yum/brew for Kubernetes"
- Packages Kubernetes manifests into reusable "charts"
- Handles templating and configuration management
- Makes it easy to install complex applications (databases, monitoring, etc.)

**When to use Helm:**
- Installing third-party applications (PostgreSQL, Redis, Prometheus)
- Managing multiple environments (dev, staging, prod)
- Packaging your own applications for reuse

**Why we're not using it in this tutorial:**
- We want to understand Kubernetes manifests directly
- Raft implementation is simple enough to manage with raw YAML
- Learning Helm adds complexity without much benefit for this project

**Documentation**: https://helm.sh/docs/

---

## Step 6: Install k9s (Optional but Highly Recommended)

k9s is a terminal-based UI for managing Kubernetes clusters. It's like a dashboard in your terminal.

```bash
# Install via Homebrew
brew install k9s

# Launch it
k9s

# You'll see a real-time view of your cluster
```

**k9s Navigation:**

| Key | Action |
|-----|--------|
| `:pod` | View pods |
| `:svc` | View services |
| `:deploy` | View deployments |
| `:ns` | View namespaces |
| `0` | Show all namespaces |
| `↑/↓` | Navigate resources |
| `Enter` | Describe resource |
| `l` | View logs |
| `d` | Delete resource |
| `e` | Edit resource |
| `s` | Shell into pod |
| `y` | View YAML |
| `?` | Help |
| `Ctrl+C` or `q` | Exit |

**Why k9s is useful:**
- Real-time view of all cluster resources
- Color-coded status (green=running, red=error, yellow=pending)
- Quickly view logs without typing kubectl commands
- Easy navigation between related resources
- Shows resource usage (CPU, memory)
- Keyboard shortcuts for common operations

**Alternatives:**
- **Lens**: https://k8slens.dev/ - Full GUI application (heavier but feature-rich)
- **kubectl dashboard**: Official web UI (rarely used for development)
- **Octant**: https://octant.dev/ - Web-based dashboard (development stopped)

**Documentation**: https://k9scli.io/

---

## Step 7: Verify Your Complete Setup

Let's run a comprehensive check to ensure everything is working together.

```bash
# 1. Check Rust toolchain
echo "=== Rust Toolchain ==="
cargo --version
rustc --version
echo

# 2. Check Docker
echo "=== Docker ==="
docker --version
docker ps
echo

# 3. Check Kubernetes cluster
echo "=== Kubernetes Cluster ==="
kubectl cluster-info
kubectl get nodes
kubectl get namespaces
echo

# 4. Check kubectl configuration
echo "=== kubectl Configuration ==="
kubectl config current-context
kubectl config get-contexts
echo

# 5. Optional: Check k9s and helm
echo "=== Optional Tools ==="
helm version 2>/dev/null || echo "Helm not installed (optional)"
which k9s >/dev/null && echo "k9s installed" || echo "k9s not installed (optional)"
```

**Expected output summary:**
- Rust: 1.75+ (or whatever latest stable is)
- Docker: 4.36.0+
- Kubernetes: v1.31.x (or latest Docker Desktop version)
- kubectl context: `docker-desktop`
- Nodes: 1 node named `docker-desktop` with status `Ready`

**Deploy a test pod to verify everything works:**

```bash
# Create a test nginx pod
kubectl run test-nginx --image=nginx:alpine --port=80

# Wait for it to start
kubectl wait --for=condition=ready pod/test-nginx --timeout=60s

# Check it's running
kubectl get pod test-nginx
# Should show: STATUS Running

# Get more details
kubectl describe pod test-nginx

# Check logs
kubectl logs test-nginx

# Test port forwarding
kubectl port-forward test-nginx 8080:80 &
curl http://localhost:8080
# Should show nginx welcome page HTML

# Clean up
kubectl delete pod test-nginx
# Kill the port-forward background process
pkill -f "port-forward test-nginx"
```

If all of these commands work, your environment is ready!

---

## Common Issues and Solutions

### Issue: Docker Desktop won't start
**Symptoms**: Docker icon shows error, can't run `docker ps`

**Solutions**:
1. Check if another VM software is running (VirtualBox, VMware) - might conflict
2. Restart Mac
3. Reset Docker Desktop: Settings → Troubleshoot → Reset to factory defaults
4. Check System Settings → Privacy & Security → ensure Docker has Full Disk Access

### Issue: Kubernetes won't enable
**Symptoms**: Checkbox enabled but cluster never starts

**Solutions**:
1. Check Docker resource allocation (needs 4+ CPU, 4+ GB RAM minimum)
2. Reset Kubernetes: Settings → Kubernetes → Reset Kubernetes Cluster
3. Check if port 6443 is already in use: `lsof -i :6443`
4. Try disabling, restarting Docker Desktop, then re-enabling

### Issue: kubectl command not found
**Symptoms**: `kubectl: command not found`

**Solutions**:
```bash
# Check if Docker Desktop installed it
ls -l /usr/local/bin/kubectl

# If missing, install separately
brew install kubectl

# Add to PATH if needed
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Issue: kubectl can't connect to cluster
**Symptoms**: `The connection to the server localhost:8080 was refused`

**Solutions**:
```bash
# Check current context
kubectl config current-context

# Should show: docker-desktop
# If not, switch to it:
kubectl config use-context docker-desktop

# Check if Kubernetes is actually running in Docker Desktop
docker ps | grep k8s

# If no k8s containers, Kubernetes might not be enabled
# Go to Docker Desktop → Settings → Kubernetes → Enable Kubernetes
```

### Issue: Pods stuck in "Pending" state
**Symptoms**: `kubectl get pods` shows pods with STATUS Pending

**Solutions**:
```bash
# Check why it's pending
kubectl describe pod <pod-name>
# Look at "Events" section at the bottom

# Common causes:
# 1. Not enough resources - increase Docker Desktop memory/CPU
# 2. Image pull error - check image name and internet connection
# 3. PVC not binding - check persistent volume claims

# Check node resources
kubectl describe node docker-desktop
# Look at "Allocated resources" section
```

### Issue: Permission denied on Docker commands
**Symptoms**: `permission denied while trying to connect to the Docker daemon socket`

**Solutions**:
```bash
# On macOS this usually means Docker Desktop isn't running
# Start Docker Desktop from Applications

# Verify it's running
docker info

# On Linux, you might need to add user to docker group
# (Not applicable on macOS)
```

---

## Resource Monitoring

It's helpful to monitor resource usage, especially when running multiple Raft nodes.

**Docker Desktop has a built-in resource monitor:**
- Click Docker icon in menu bar → Dashboard
- Click "Resources" tab
- Shows CPU, memory, disk usage in real-time

**Terminal-based monitoring:**

```bash
# Docker stats (like top for containers)
docker stats

# Kubernetes resource usage
kubectl top nodes  # Requires metrics-server (not in Docker Desktop by default)
kubectl top pods   # Same

# System monitoring
# Activity Monitor app (GUI)
# Or use htop (install: brew install htop)
htop
```

---

## Next Steps

Your development environment is now fully configured! You have:

✅ Rust toolchain for building performant systems applications
✅ Docker for containerizing your Raft nodes
✅ Kubernetes for orchestrating a distributed cluster
✅ kubectl for managing Kubernetes resources
✅ Optional tools (k9s, helm) for enhanced productivity

**In the next sections**, we'll dive deep into:
- **Tokio**: Async runtime for handling concurrent network connections
- **Axum**: Web framework for HTTP and WebSocket communication
- **Serde**: Serialization for Raft messages
- **Clap**: Configuration management for your nodes

Each of these tools will be essential for building your Raft implementation!

**Continue to**: [Part 1: Understanding Tokio](01-tokio.md)
