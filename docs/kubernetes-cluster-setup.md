# Kubernetes Multi-Node Cluster Setup for Whitewater

## Overview

This guide explains how to deploy Whitewater as a multi-node Raft cluster in Kubernetes. Each pod needs to:

1. **Know its own identity** - pod name and IP address
2. **Discover all peers** - find every other pod in the cluster
3. **Communicate with peers** - establish connections for Raft consensus

This document covers three approaches for implementing this, with emphasis on Kubernetes mechanics and configuration.

---

## Option 1: DNS SRV Query

### Overview

Pods query Kubernetes DNS at runtime to discover all peers via SRV records. Kubernetes maintains these records automatically as pods are created or destroyed.

### Kubernetes Configuration

#### Headless Service

A headless service (without a cluster IP) creates DNS records for individual pods rather than load balancing to them.

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whitewater-headless
  namespace: default
spec:
  clusterIP: None              # Headless service
  selector:
    app: whitewater
  ports:
  - port: 8090
    name: raft                 # Named port creates SRV record
```

**What this creates:**

When pods are running, Kubernetes DNS provides:
- **A records** for each pod: `whitewater-0.whitewater-headless.default.svc.cluster.local` → pod IP
- **SRV record** for the service: `_raft._tcp.whitewater-headless.default.svc.cluster.local` → list of all pod addresses

The SRV record is key for discovery. Query it to get all running pods.

#### StatefulSet

StatefulSets provide stable pod names and ordering. Unlike Deployments, pods get predictable names like `whitewater-0`, `whitewater-1`, etc.

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: whitewater
  namespace: default
spec:
  replicas: 5
  serviceName: whitewater-headless  # Links to headless service for DNS
  selector:
    matchLabels:
      app: whitewater
  template:
    metadata:
      labels:
        app: whitewater
    spec:
      containers:
      - name: whitewater
        image: whitewater:1
        imagePullPolicy: Never
        ports:
        - containerPort: 8090
          name: raft
        env:
        # Info needed to query DNS
        - name: SERVICE_NAME
          value: "whitewater-headless"
        - name: SERVICE_PORT_NAME
          value: "raft"
        - name: NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        # Pod's own identity via Downward API
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_IP
          valueFrom:
            fieldRef:
              fieldPath: status.podIP

---
# Regular service for external access
apiVersion: v1
kind: Service
metadata:
  name: whitewater-service
  namespace: default
spec:
  type: ClusterIP
  selector:
    app: whitewater
  ports:
  - port: 8090
    targetPort: 8090
```

**Key Kubernetes concepts:**

- **StatefulSet** creates pods sequentially with stable names
- **serviceName** field links to the headless service
- **Downward API** (`fieldRef`) injects pod metadata as environment variables
- Pods can read their own name/IP from env vars
- Pods can query DNS SRV to find peers

### How DNS SRV Works

When you query `_raft._tcp.whitewater-headless.default.svc.cluster.local`, you get:

```
_raft._tcp.whitewater-headless.default.svc.cluster.local. 30 IN SRV 0 20 8090 whitewater-0.whitewater-headless.default.svc.cluster.local.
_raft._tcp.whitewater-headless.default.svc.cluster.local. 30 IN SRV 0 20 8090 whitewater-1.whitewater-headless.default.svc.cluster.local.
_raft._tcp.whitewater-headless.default.svc.cluster.local. 30 IN SRV 0 20 8090 whitewater-2.whitewater-headless.default.svc.cluster.local.
...
```

SRV record format: `priority weight port hostname`

Each entry tells you a pod's hostname and port. Query these hostnames to get IPs.

### Rust Implementation

**Cargo.toml:**
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
hickory-resolver = "0.24"
anyhow = "1.0"
```

**src/main.rs:**
```rust
use hickory_resolver::AsyncResolver;
use hickory_resolver::config::*;
use std::env;
use anyhow::Result;

async fn discover_peers() -> Result<Vec<String>> {
    let service = env::var("SERVICE_NAME")?;
    let namespace = env::var("NAMESPACE")?;
    let port_name = env::var("SERVICE_PORT_NAME")?;

    let resolver = AsyncResolver::tokio(
        ResolverConfig::default(),
        ResolverOpts::default(),
    );

    let srv_query = format!("_{}._{}.{}.{}.svc.cluster.local",
        port_name, "tcp", service, namespace);

    let records = resolver.srv_lookup(&srv_query).await?;

    let peers = records.iter()
        .map(|srv| format!("{}:{}", srv.target().to_utf8(), srv.port()))
        .collect();

    Ok(peers)
}

#[tokio::main]
async fn main() -> Result<()> {
    let my_name = env::var("POD_NAME")?;
    let my_ip = env::var("POD_IP")?;
    let peers = discover_peers().await?;

    println!("I am: {} ({})", my_name, my_ip);
    println!("Peers: {:?}", peers);

    // TODO: Connect to peers and start Raft
    Ok(())
}
```

### Scaling

```bash
kubectl scale statefulset whitewater --replicas=7
```

New pods appear in DNS automatically. To pick up changes, either:
- Restart pods
- Implement periodic DNS re-query in your application

### Pros & Cons

**Pros:**
- Built into Kubernetes (no extra setup)
- Lightweight dependency (just DNS resolver)
- DNS is fast and reliable

**Cons:**
- Requires DNS resolver library in Rust
- Need to poll DNS or restart to detect changes
- DNS queries are network calls (though local)

---

## Option 2: Kubernetes API Query

### Overview

Pods query the Kubernetes API directly to list all pods with a specific label. Requires RBAC setup but provides real-time updates via watch API.

### Kubernetes Configuration

#### RBAC Setup

By default, pods cannot access the Kubernetes API. You must create:
1. **ServiceAccount** - identity for your pods
2. **Role** - permissions to read pods
3. **RoleBinding** - grants the role to the service account

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: whitewater
  namespace: default

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: whitewater-pod-reader
  namespace: default
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: whitewater-pod-reader-binding
  namespace: default
subjects:
- kind: ServiceAccount
  name: whitewater
  namespace: default
roleRef:
  kind: Role
  name: whitewater-pod-reader
  apiGroup: rbac.authorization.k8s.io
```

**How RBAC works:**

- Kubernetes creates a token for the ServiceAccount
- Token is mounted at `/var/run/secrets/kubernetes.io/serviceaccount/token`
- When pods make API requests, they include this token
- Kubernetes checks the token against Role permissions
- If allowed, request succeeds

#### Services and StatefulSet

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whitewater-headless
  namespace: default
spec:
  clusterIP: None
  selector:
    app: whitewater
  ports:
  - port: 8090
    name: raft

---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: whitewater
  namespace: default
spec:
  replicas: 5
  serviceName: whitewater-headless
  selector:
    matchLabels:
      app: whitewater
  template:
    metadata:
      labels:
        app: whitewater
    spec:
      serviceAccountName: whitewater  # Use the ServiceAccount we created
      containers:
      - name: whitewater
        image: whitewater:1
        imagePullPolicy: Never
        ports:
        - containerPort: 8090
          name: raft
        env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_IP
          valueFrom:
            fieldRef:
              fieldPath: status.podIP
        - name: NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        - name: LABEL_SELECTOR
          value: "app=whitewater"

---
apiVersion: v1
kind: Service
metadata:
  name: whitewater-service
  namespace: default
spec:
  type: ClusterIP
  selector:
    app: whitewater
  ports:
  - port: 8090
    targetPort: 8090
```

### How API Access Works

1. Pod starts with ServiceAccount token mounted
2. Rust client (`kube-rs`) reads the token
3. Client makes request: `GET /api/v1/namespaces/default/pods?labelSelector=app=whitewater`
4. Kubernetes validates token against RBAC
5. API returns JSON list of matching pods
6. Client extracts names, IPs, and metadata

### Rust Implementation

**Cargo.toml:**
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
kube = { version = "0.96", features = ["client", "runtime"] }
k8s-openapi = { version = "0.23", features = ["v1_30"] }
anyhow = "1.0"
```

**src/main.rs:**
```rust
use kube::{Api, Client};
use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;
use std::env;
use anyhow::Result;

async fn discover_peers() -> Result<Vec<(String, String)>> {
    let namespace = env::var("NAMESPACE")?;
    let label_selector = env::var("LABEL_SELECTOR")?;

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    let lp = ListParams::default().labels(&label_selector);
    let pod_list = pods.list(&lp).await?;

    let peers = pod_list.items.iter()
        .filter_map(|pod| {
            let name = pod.metadata.name.as_ref()?;
            let ip = pod.status.as_ref()?.pod_ip.as_ref()?;
            Some((name.clone(), ip.clone()))
        })
        .collect();

    Ok(peers)
}

#[tokio::main]
async fn main() -> Result<()> {
    let my_name = env::var("POD_NAME")?;
    let my_ip = env::var("POD_IP")?;
    let peers = discover_peers().await?;

    println!("I am: {} ({})", my_name, my_ip);
    println!("Peers:");
    for (name, ip) in &peers {
        println!("  {} ({})", name, ip);
    }

    // TODO: Connect to peers and start Raft
    Ok(())
}
```

### Watch API for Real-Time Updates

The Kubernetes API supports "watch" mode to stream pod changes:

```rust
use kube::runtime::watcher::{watcher, Config, Event};
use futures::StreamExt;

async fn watch_peers() -> Result<()> {
    let namespace = env::var("NAMESPACE")?;
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    let mut stream = watcher(pods, Config::default()).boxed();

    while let Some(event) = stream.next().await {
        match event? {
            Event::Apply(pod) => println!("Pod added/modified: {:?}", pod.metadata.name),
            Event::Delete(pod) => println!("Pod deleted: {:?}", pod.metadata.name),
            _ => {}
        }
        // TODO: Update Raft cluster configuration
    }

    Ok(())
}
```

### Scaling

```bash
kubectl scale statefulset whitewater --replicas=7
```

With watch mode, all pods receive events immediately and can update their peer lists without restart.

### Pros & Cons

**Pros:**
- Real-time updates via watch API
- Access to full pod metadata (readiness, labels, annotations)
- Can filter by readiness status

**Cons:**
- Requires RBAC configuration
- Larger dependency (kube-rs)
- Uses pod IPs directly (less stable than DNS names)
- More complex than DNS

---

## Option 3: Init Container Discovery

### Overview

An init container runs before the main application starts, discovers peers, and writes them to a shared volume. The main application reads the file.

This separates discovery logic from application logic and minimizes dependencies in the main container.

### Kubernetes Configuration

#### Init Container Approach

```yaml
apiVersion: v1
kind: Service
metadata:
  name: whitewater-headless
  namespace: default
spec:
  clusterIP: None
  selector:
    app: whitewater
  ports:
  - port: 8090
    name: raft

---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: whitewater
  namespace: default
spec:
  replicas: 5
  serviceName: whitewater-headless
  selector:
    matchLabels:
      app: whitewater
  template:
    metadata:
      labels:
        app: whitewater
    spec:
      # Init container runs first
      initContainers:
      - name: discover-peers
        image: busybox:1.36
        command:
        - sh
        - -c
        - |
          echo "Discovering peers via DNS..."

          # Query SRV record
          nslookup -type=SRV _raft._tcp.whitewater-headless.default.svc.cluster.local > /tmp/srv.txt || true

          # Extract hostnames from SRV records
          PEERS=$(grep "whitewater-" /tmp/srv.txt | awk '{print $NF}' | sed 's/\.$//')

          # Write to JSON file
          echo '{"peers":[' > /config/peers.json
          FIRST=1
          for PEER in $PEERS; do
            [ $FIRST -eq 0 ] && echo ',' >> /config/peers.json
            FIRST=0
            echo "\"$PEER:8090\"" | tr -d '\n' >> /config/peers.json
          done
          echo ']}' >> /config/peers.json

          echo "Discovered peers:"
          cat /config/peers.json
        volumeMounts:
        - name: config
          mountPath: /config

      # Main application container
      containers:
      - name: whitewater
        image: whitewater:1
        imagePullPolicy: Never
        ports:
        - containerPort: 8090
          name: raft
        env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_IP
          valueFrom:
            fieldRef:
              fieldPath: status.podIP
        - name: PEERS_FILE
          value: "/config/peers.json"
        volumeMounts:
        - name: config
          mountPath: /config
          readOnly: true

      volumes:
      - name: config
        emptyDir: {}  # Temporary shared volume

---
apiVersion: v1
kind: Service
metadata:
  name: whitewater-service
  namespace: default
spec:
  type: ClusterIP
  selector:
    app: whitewater
  ports:
  - port: 8090
    targetPort: 8090
```

### How Init Containers Work

**Execution order:**
1. Pod starts
2. `emptyDir` volume created (empty)
3. Init container runs:
   - Queries DNS
   - Writes `/config/peers.json`
   - Exits
4. Main container starts:
   - Reads `/config/peers.json`
   - Starts Raft with discovered peers

**Key properties:**
- Init containers always run before app containers
- They run sequentially (if multiple)
- Must complete successfully or pod fails
- Share volumes with main container
- `emptyDir` persists for pod lifetime

### Rust Implementation

**Cargo.toml:**
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

**src/main.rs:**
```rust
use serde::Deserialize;
use std::env;
use std::fs;
use anyhow::Result;

#[derive(Deserialize)]
struct PeersConfig {
    peers: Vec<String>,
}

fn load_peers() -> Result<Vec<String>> {
    let file_path = env::var("PEERS_FILE")?;
    let content = fs::read_to_string(file_path)?;
    let config: PeersConfig = serde_json::from_str(&content)?;
    Ok(config.peers)
}

#[tokio::main]
async fn main() -> Result<()> {
    let my_name = env::var("POD_NAME")?;
    let my_ip = env::var("POD_IP")?;
    let peers = load_peers()?;

    println!("I am: {} ({})", my_name, my_ip);
    println!("Peers: {:?}", peers);

    // TODO: Connect to peers and start Raft
    Ok(())
}
```

### Alternative: Init Container with K8s API

You can use the Kubernetes API in the init container instead of DNS. This requires adding RBAC (same as Option 2) and using a different init container image:

```yaml
initContainers:
- name: discover-peers
  image: bitnami/kubectl:latest
  command:
  - sh
  - -c
  - |
    NAMESPACE=$(cat /var/run/secrets/kubernetes.io/serviceaccount/namespace)
    kubectl get pods -n $NAMESPACE -l app=whitewater -o jsonpath='{range .items[*]}{.status.podIP}{"\n"}{end}' | \
    awk 'BEGIN {print "{\"peers\":["} {printf "%s\"%s:8090\"", (NR>1?",":""), $0} END {print "]}"}' > /config/peers.json
    cat /config/peers.json
  volumeMounts:
  - name: config
    mountPath: /config
```

### Scaling

```bash
kubectl scale statefulset whitewater --replicas=7
```

New pods discover current peers at startup. Existing pods won't see new peers until they restart.

To force update:
```bash
kubectl rollout restart statefulset whitewater
```

### Pros & Cons

**Pros:**
- Minimal dependencies in main application (just JSON parsing)
- Discovery logic isolated in init container
- Can use any tool/language for discovery
- Clear separation of concerns

**Cons:**
- Not dynamic (requires pod restart to update peers)
- Slightly slower startup (init container must complete first)
- More complex YAML
- Additional container image to maintain

---

## Comparison

| Feature | DNS SRV | K8s API | Init Container |
|---------|---------|---------|----------------|
| **Discovery method** | DNS query | API query | Init script → file |
| **Dynamic updates** | Polling | Watch streams | Restart only |
| **K8s setup** | Headless Service | + RBAC | Headless Service (or + RBAC) |
| **Rust dependencies** | hickory-resolver | kube + k8s-openapi | serde_json |
| **Startup time** | Fast | Fast | Slower (init runs first) |
| **Complexity** | Low | Medium | Medium |

---

## Recommendations

### Use DNS SRV when:
- You want simple, standard Kubernetes setup
- Your cluster size changes occasionally
- You can poll DNS periodically or restart on scale events
- You want minimal configuration

### Use K8s API when:
- You need real-time cluster membership updates
- You want to filter by pod readiness
- You need pod metadata (labels, annotations)
- You're building dynamic, auto-scaling clusters

### Use Init Container when:
- You want to minimize main application dependencies
- Your cluster membership is relatively stable
- You want clear separation between discovery and application logic
- You're okay with pod restarts for membership changes

---

## Testing

### Verify DNS SRV

```bash
# From inside a pod
kubectl exec -it whitewater-0 -- nslookup -type=SRV _raft._tcp.whitewater-headless.default.svc.cluster.local
```

### Verify RBAC

```bash
kubectl auth can-i list pods --as=system:serviceaccount:default:whitewater
# Should output: yes
```

### Verify Init Container

```bash
# Check init container logs
kubectl logs whitewater-0 -c discover-peers

# Check generated file
kubectl exec whitewater-0 -- cat /config/peers.json
```

### Test Scaling

```bash
# Scale up
kubectl scale statefulset whitewater --replicas=7

# Watch pods
kubectl get pods -w

# Check discovery in new pod
kubectl logs whitewater-6
```

---

## Next Steps

1. Choose an approach based on your requirements
2. Deploy the Kubernetes manifests
3. Implement Raft consensus using discovered peers
4. Add health/readiness probes
5. Test failure scenarios (pod crashes, network partitions)
6. Add PersistentVolumeClaims if you need durable storage for Raft logs

## Resources

- [Kubernetes StatefulSets](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/)
- [Headless Services](https://kubernetes.io/docs/concepts/services-networking/service/#headless-services)
- [DNS for Services and Pods](https://kubernetes.io/docs/concepts/services-networking/dns-pod-service/)
- [RBAC Authorization](https://kubernetes.io/docs/reference/access-authn-authz/rbac/)
- [Init Containers](https://kubernetes.io/docs/concepts/workloads/pods/init-containers/)
