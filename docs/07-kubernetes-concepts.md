# Part 7: Kubernetes Concepts for Distributed Systems

## Table of Contents

- [What is Kubernetes?](#what-is-kubernetes)
- [Kubernetes Architecture](#kubernetes-architecture)
- [Core Resource Types](#core-resource-types)
- [Workload Resources](#workload-resources)
- [Networking](#networking)
- [Storage](#storage)
- [Configuration](#configuration)
- [Why These Concepts Matter for Distributed Systems](#why-these-concepts-matter-for-distributed-systems)

---

## What is Kubernetes?

Kubernetes (K8s) is an open-source **container orchestration platform**. Let's break that down:

### Container Orchestration

**Orchestration** means coordinating multiple containers across multiple machines to work together as a system. This includes:

1. **Scheduling**: Deciding which machine runs which container
2. **Scaling**: Adding or removing container instances based on load
3. **Self-healing**: Restarting failed containers automatically
4. **Load balancing**: Distributing traffic across container instances
5. **Service discovery**: Helping containers find each other
6. **Configuration management**: Providing secrets, config files, etc.
7. **Rolling updates**: Deploying new versions without downtime

### Why Kubernetes Exists

**Without Kubernetes**, running containers in production requires:
- Manually starting containers on each machine
- Manually restarting failed containers
- Manually distributing load
- Manually tracking which containers run where
- Scripts and glue code to automate all of this

**With Kubernetes**, you describe the **desired state** (declarative), and Kubernetes makes it happen:
- "I want 3 replicas of this container running"
- Kubernetes ensures 3 are always running
- If one crashes, Kubernetes starts a new one automatically
- If a machine fails, Kubernetes reschedules containers to healthy machines

### The Declarative Model

This is fundamental to understanding Kubernetes:

**Imperative** (Docker Compose, shell scripts):
```bash
docker run my-app      # Do this specific action
docker stop my-app     # Do another specific action
```

**Declarative** (Kubernetes):
```yaml
replicas: 3            # I want this state to be true
image: my-app:v2       # Always
```

Kubernetes has **controllers** that continuously work to make the actual state match the desired state.

**Documentation**:
- Kubernetes overview: https://kubernetes.io/docs/concepts/overview/
- What is Kubernetes: https://kubernetes.io/docs/concepts/overview/what-is-kubernetes/

---

## Kubernetes Architecture

Understanding the architecture helps you debug problems and understand what's happening.

### Cluster Components

A Kubernetes cluster has two types of machines:

#### Control Plane (Master Nodes)

The "brains" of the cluster. Components:

1. **kube-apiserver**
   - The front door to Kubernetes
   - All communication goes through this (kubectl, controllers, etc.)
   - RESTful API over HTTPS
   - Validates and persists resources to etcd

2. **etcd**
   - Distributed key-value store
   - Stores ALL cluster state (pods, services, secrets, etc.)
   - Kubernetes is stateless - if etcd is lost, cluster state is gone
   - Uses Raft consensus! (Same algorithm you're implementing)

3. **kube-scheduler**
   - Decides which node should run each pod
   - Considers: resource requirements, affinity rules, taints/tolerations
   - Doesn't actually start pods (kubelet does that)

4. **kube-controller-manager**
   - Runs multiple controllers (loops that watch state and act)
   - Examples:
     - ReplicaSet controller: Ensures N replicas are running
     - Node controller: Detects when nodes die
     - Endpoints controller: Populates Service endpoints
   - Each controller is a simple loop:
     ```
     loop {
       current_state = get_from_apiserver()
       desired_state = get_from_apiserver()
       if current_state != desired_state {
         take_action()
       }
       sleep(interval)
     }
     ```

5. **cloud-controller-manager** (optional)
   - Integrates with cloud providers (AWS, GCP, Azure)
   - Creates load balancers, volumes, etc.

#### Worker Nodes

The "muscles" that actually run containers. Components:

1. **kubelet**
   - Agent that runs on each node
   - Watches the API server for pods assigned to this node
   - Tells the container runtime (Docker, containerd) to start/stop containers
   - Reports pod status back to API server

2. **kube-proxy**
   - Manages networking rules on the node
   - Implements Service abstraction (load balancing)
   - Uses iptables or IPVS to route traffic

3. **Container Runtime**
   - Actually runs containers (Docker, containerd, CRI-O)
   - kubelet tells it what to do via the CRI (Container Runtime Interface)

### How It All Works Together

Example: You run `kubectl apply -f pod.yaml`

1. kubectl sends HTTP POST to kube-apiserver with pod definition
2. kube-apiserver validates the pod spec
3. kube-apiserver writes pod to etcd (status: Pending)
4. kube-scheduler watches API server, sees new pod with no node assigned
5. kube-scheduler decides which node should run the pod
6. kube-scheduler updates pod in API server (nodeName: worker-1)
7. kubelet on worker-1 watches API server, sees pod assigned to itself
8. kubelet tells container runtime to pull image and start container
9. kubelet updates pod status in API server (status: Running)

**Everything happens through the API server. etcd is the only stateful component.**

**Documentation**:
- Architecture: https://kubernetes.io/docs/concepts/architecture/
- Components: https://kubernetes.io/docs/concepts/overview/components/

---

## Core Resource Types

Kubernetes is built around **resources** (also called **objects** or **kinds**). Each resource type serves a specific purpose.

### Understanding Resources

Every Kubernetes resource has:

1. **apiVersion**: Which version of the Kubernetes API this resource uses
   - `v1`: Core resources (Pod, Service, etc.)
   - `apps/v1`: App-related resources (Deployment, StatefulSet)
   - `batch/v1`: Job-related resources

2. **kind**: The type of resource (Pod, Service, Deployment, etc.)

3. **metadata**: Information about the resource
   - `name`: Unique identifier within namespace
   - `namespace`: Logical grouping (like a folder)
   - `labels`: Key-value pairs for grouping and selection
   - `annotations`: Non-identifying metadata (notes, URLs, etc.)

4. **spec**: The desired state (what you want)
   - Different for each resource type

5. **status**: The actual current state (what actually is)
   - Managed by Kubernetes, not you
   - Useful for debugging

### Namespaces

**What**: Logical partitions within a cluster

**Why**:
- Isolation (dev vs staging vs prod)
- Multi-tenancy (team-a vs team-b)
- Resource quotas per namespace
- RBAC (access control) per namespace

**Default namespaces**:
- `default`: Where resources go if you don't specify
- `kube-system`: Kubernetes system components
- `kube-public`: Public resources (readable by all)
- `kube-node-lease`: Node heartbeat data (internal)

**Example**:
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: raft-cluster
```

**DNS**: Resources in different namespaces have different FQDNs:
- `service-name` (within same namespace)
- `service-name.namespace-name`
- `service-name.namespace-name.svc.cluster.local` (full FQDN)

**Documentation**: https://kubernetes.io/docs/concepts/overview/working-with-objects/namespaces/

### Labels and Selectors

**Labels** are key-value pairs attached to resources:
```yaml
metadata:
  labels:
    app: raft
    component: node
    environment: production
```

**Selectors** are queries to find resources by labels:
```yaml
selector:
  matchLabels:
    app: raft       # Find all resources with app=raft
  matchExpressions:
    - key: environment
      operator: In
      values: [production, staging]
```

**Why labels matter**:
- Services use selectors to find Pods
- ReplicaSets use selectors to count how many Pods exist
- You use selectors to query: `kubectl get pods -l app=raft`

**Common label keys**:
- `app`: Application name
- `component`: Role within app (frontend, backend, database)
- `version`: Version of the app
- `environment`: dev, staging, production

**Documentation**: https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/

---

## Workload Resources

These resources manage how containers run.

### Pod

**What**: The smallest deployable unit in Kubernetes. A pod is:
- One or more containers that share:
  - Network namespace (same IP, can use localhost)
  - IPC namespace (can communicate via shared memory)
  - UTS namespace (same hostname)
  - Optional: storage volumes
- Scheduled together on the same node (atomic unit)

**Why multiple containers in a pod?**
- Main container + sidecar containers
- Example: App container + logging agent + metrics exporter
- Tightly coupled containers that need to share resources

**Lifecycle**:
1. Pending: Waiting to be scheduled or images to be pulled
2. Running: At least one container is running
3. Succeeded: All containers exited with status 0
4. Failed: At least one container exited with non-zero status
5. Unknown: Can't determine state (node communication lost)

**Pods are ephemeral**:
- Pods die and are replaced (not restarted)
- When a pod dies, you get a NEW pod with a NEW IP
- Don't rely on pod IPs - use Services for stable networking

**Example**:
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: my-pod
spec:
  containers:
  - name: app
    image: my-app:v1
    ports:
    - containerPort: 8080
```

**Documentation**: https://kubernetes.io/docs/concepts/workloads/pods/

### ReplicaSet

**What**: Ensures a specified number of pod replicas are running

**How it works**:
1. You specify: `replicas: 3` and a pod template
2. ReplicaSet controller continuously counts matching pods (via selector)
3. If count < 3: Create new pods
4. If count > 3: Delete excess pods
5. If count == 3: Do nothing

**You rarely create ReplicaSets directly** - Deployments manage them for you.

**Example**:
```yaml
apiVersion: apps/v1
kind: ReplicaSet
metadata:
  name: my-replicaset
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
      - name: app
        image: my-app:v1
```

**Documentation**: https://kubernetes.io/docs/concepts/workloads/controllers/replicaset/

### Deployment

**What**: Declarative updates for Pods and ReplicaSets

**Why use Deployment instead of ReplicaSet?**
- Rolling updates: Update pods gradually (zero downtime)
- Rollback: If update fails, rollback to previous version
- Pause/resume: Stop updates mid-roll
- Revision history: Track changes over time

**How rolling updates work**:
1. You update the Deployment (change image from v1 to v2)
2. Deployment creates a NEW ReplicaSet for v2 pods
3. Deployment gradually scales up v2 ReplicaSet, scales down v1 ReplicaSet
4. Old pods are terminated, new pods are created
5. Controlled by `maxSurge` and `maxUnavailable` settings

**Example**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-deployment
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
      - name: app
        image: my-app:v1
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1          # Max 1 extra pod during update
      maxUnavailable: 0    # Min 3 pods always available
```

**Use Deployment for**: Stateless applications (web servers, APIs)

**Documentation**: https://kubernetes.io/docs/concepts/workloads/controllers/deployment/

### StatefulSet

**What**: Like a Deployment, but for stateful applications

**Key differences from Deployment**:

1. **Stable network identities**
   - Pods get predictable names: `statefulset-name-0`, `statefulset-name-1`, `statefulset-name-2`
   - Names don't change when pods are rescheduled
   - Each pod gets a stable DNS name

2. **Ordered deployment and scaling**
   - Pods are created in order: 0, then 1, then 2
   - Pods are deleted in reverse order: 2, then 1, then 0
   - Pod N is not created until Pod N-1 is Running and Ready

3. **Persistent storage per pod**
   - Each pod gets its own PersistentVolumeClaim
   - Storage persists across pod rescheduling
   - Pod `raft-0` always mounts the same volume

**DNS for StatefulSets** (with Headless Service):
```
<pod-name>.<service-name>.<namespace>.svc.cluster.local

raft-0.raft-headless.default.svc.cluster.local
raft-1.raft-headless.default.svc.cluster.local
raft-2.raft-headless.default.svc.cluster.local
```

**Example**:
```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: raft
spec:
  serviceName: raft-headless  # Must match a Headless Service
  replicas: 3
  selector:
    matchLabels:
      app: raft
  template:
    metadata:
      labels:
        app: raft
    spec:
      containers:
      - name: raft
        image: my-raft:v1
        ports:
        - containerPort: 9000
        volumeMounts:
        - name: data
          mountPath: /app/data
  volumeClaimTemplates:  # Creates PVC for each pod
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 1Gi
```

**Use StatefulSet for**:
- Databases (PostgreSQL, MongoDB, Cassandra)
- Distributed coordination systems (ZooKeeper, etcd, **Raft**)
- Message queues (Kafka, RabbitMQ)
- Any application that needs stable network IDs or persistent storage

**Documentation**: https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/

---

## Networking

Kubernetes networking is complex. Here are the core concepts:

### Pod Networking

**Fundamental rule**: Every pod gets its own IP address

- Pods can communicate with each other directly (no NAT)
- All pods are on a flat network (can reach any other pod)
- Implemented by CNI (Container Network Interface) plugins
  - Common plugins: Calico, Flannel, Weave, Cilium

**How it works** (simplified):
1. Each node has a subnet (e.g., 10.244.0.0/24, 10.244.1.0/24)
2. Pods on a node get IPs from that node's subnet
3. CNI plugin routes traffic between nodes
4. Pod-to-pod traffic works across nodes

**Problem**: Pod IPs change when pods are rescheduled. How do you connect to a pod reliably?

**Solution**: Services (next section)

### Service

**What**: A stable network endpoint for a set of pods

**Why**: Pods are ephemeral (IPs change), Services are stable

**How it works**:
1. You create a Service with a selector (e.g., `app=raft`)
2. Service controller finds all matching pods
3. Service gets a stable Cluster IP (e.g., 10.96.0.10)
4. kube-proxy configures iptables rules to load balance to pod IPs
5. When pods are added/removed, iptables rules are updated
6. Clients connect to the Service IP, traffic is routed to healthy pods

**Service Types**:

1. **ClusterIP** (default)
   - Service is accessible only within the cluster
   - Gets a virtual IP from the service CIDR (e.g., 10.96.0.0/12)
   - Use for: Internal microservices

2. **NodePort**
   - Exposes service on each node's IP at a static port (30000-32767)
   - `<NodeIP>:<NodePort>` is accessible from outside the cluster
   - ClusterIP is also created automatically
   - Use for: Development, simple external access

3. **LoadBalancer**
   - Creates an external load balancer (cloud provider specific)
   - Assigns an external IP
   - NodePort and ClusterIP are also created automatically
   - Use for: Production external access (requires cloud provider)

4. **ExternalName**
   - Maps service to a DNS name (e.g., database.example.com)
   - No proxy, just DNS CNAME
   - Use for: External services

**Example (ClusterIP)**:
```yaml
apiVersion: v1
kind: Service
metadata:
  name: raft-service
spec:
  type: ClusterIP
  selector:
    app: raft
  ports:
  - name: http
    port: 80          # Service port (what clients connect to)
    targetPort: 8080  # Container port (where app listens)
  - name: raft
    port: 9000
    targetPort: 9000
```

**DNS for Services**:
```
<service-name>.<namespace>.svc.cluster.local

raft-service.default.svc.cluster.local
```

**Documentation**: https://kubernetes.io/docs/concepts/services-networking/service/

### Headless Service

**What**: A Service without a Cluster IP

**Why**: For stateful applications that need direct pod-to-pod communication

**How it's different**:
- Set `clusterIP: None` in Service spec
- No load balancing by kube-proxy
- DNS returns A records for ALL pod IPs (not just service IP)
- Each pod gets its own DNS entry

**DNS for Headless Service**:
```
# Service DNS (returns all pod IPs)
<service-name>.<namespace>.svc.cluster.local

# Individual pod DNS
<pod-name>.<service-name>.<namespace>.svc.cluster.local

raft-0.raft-headless.default.svc.cluster.local → 10.244.1.5
raft-1.raft-headless.default.svc.cluster.local → 10.244.2.3
raft-2.raft-headless.default.svc.cluster.local → 10.244.1.7
```

**Example**:
```yaml
apiVersion: v1
kind: Service
metadata:
  name: raft-headless
spec:
  clusterIP: None      # This makes it headless
  selector:
    app: raft
  ports:
  - name: raft
    port: 9000
```

**Use for**: StatefulSets where each pod needs a stable DNS name (databases, Raft clusters)

**Documentation**: https://kubernetes.io/docs/concepts/services-networking/service/#headless-services

### Endpoints

**What**: The actual list of IP addresses behind a Service

**How it works**:
- Service controller automatically creates an Endpoints object
- Same name as the Service
- Contains IP:port pairs for all matching healthy pods
- kube-proxy watches Endpoints to update iptables rules

**Check Endpoints**:
```bash
kubectl get endpoints raft-service

NAME           ENDPOINTS
raft-service   10.244.1.5:9000,10.244.2.3:9000,10.244.1.7:9000
```

---

## Storage

Containers are ephemeral - data is lost when containers stop. Volumes provide persistence.

### Volumes

**What**: A directory accessible to containers in a pod

**Lifetime**: Same as the pod (volume outlives container restarts, but not pod deletion)

**Types** (there are many):

1. **emptyDir**: Empty directory created when pod starts, deleted when pod ends
   - Use for: Temporary caches, scratch space

2. **hostPath**: Mount a directory from the node's filesystem
   - Use for: Accessing node files (logging agents, monitoring)
   - Warning: Pods on different nodes see different data

3. **configMap**: Mount ConfigMap as files
   - Use for: Configuration files

4. **secret**: Mount Secret as files
   - Use for: Credentials, certificates

5. **persistentVolumeClaim**: Mount a PersistentVolume
   - Use for: Databases, stateful apps

**Example**:
```yaml
spec:
  containers:
  - name: app
    volumeMounts:
    - name: cache
      mountPath: /cache
  volumes:
  - name: cache
    emptyDir: {}
```

### PersistentVolume (PV)

**What**: A piece of storage in the cluster

**Created by**: Cluster administrators or dynamically by storage classes

**Lifecycle**: Independent of pods (persists beyond pod lifetime)

**Types**:
- Local: Storage on a specific node
- Network: NFS, iSCSI, Ceph, AWS EBS, GCP Persistent Disk, Azure Disk, etc.

**Example**:
```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: pv-raft-0
spec:
  capacity:
    storage: 10Gi
  accessModes:
  - ReadWriteOnce  # Can be mounted read-write by one node
  persistentVolumeReclaimPolicy: Retain
  hostPath:
    path: /mnt/data/raft-0
```

### PersistentVolumeClaim (PVC)

**What**: A request for storage by a user

**How it works**:
1. User creates a PVC: "I need 10Gi of storage"
2. Kubernetes finds a matching PV (or creates one dynamically)
3. PVC is "bound" to the PV
4. Pod mounts the PVC

**Example**:
```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: raft-data-claim
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

**In a Pod**:
```yaml
spec:
  containers:
  - name: raft
    volumeMounts:
    - name: data
      mountPath: /app/data
  volumes:
  - name: data
    persistentVolumeClaim:
      claimName: raft-data-claim
```

### Access Modes

- **ReadWriteOnce (RWO)**: One node can mount read-write
- **ReadOnlyMany (ROX)**: Many nodes can mount read-only
- **ReadWriteMany (RWX)**: Many nodes can mount read-write (requires special storage like NFS)

### StorageClass

**What**: Describes different "classes" of storage

**Why**: Different performance tiers (fast SSD, slow HDD), different providers

**Dynamic provisioning**: PVs are created automatically when PVC is created

**Example**:
```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-ssd
provisioner: kubernetes.io/aws-ebs
parameters:
  type: gp3
  iopsPerGB: "10"
```

**Documentation**: https://kubernetes.io/docs/concepts/storage/storage-classes/

---

## Configuration

### ConfigMap

**What**: Key-value pairs or files for configuration

**Use for**: Non-sensitive configuration (URLs, feature flags, config files)

**Example**:
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: raft-config
data:
  election_timeout: "300"
  heartbeat_interval: "50"
  cluster_size: "3"
  peers: "raft-0:9000,raft-1:9000,raft-2:9000"
```

**Consume in Pod** (as environment variables):
```yaml
spec:
  containers:
  - name: raft
    envFrom:
    - configMapRef:
        name: raft-config
```

**Consume as volume** (as files):
```yaml
spec:
  containers:
  - name: raft
    volumeMounts:
    - name: config
      mountPath: /etc/config
  volumes:
  - name: config
    configMap:
      name: raft-config
```

**Documentation**: https://kubernetes.io/docs/concepts/configuration/configmap/

### Secret

**What**: Like ConfigMap, but for sensitive data (base64 encoded)

**Use for**: Passwords, API keys, TLS certificates

**Example**:
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: raft-secret
type: Opaque
data:
  admin-password: YWRtaW4xMjM=  # base64 encoded
```

**Important**: Secrets are NOT encrypted by default in etcd! Use encryption at rest.

**Documentation**: https://kubernetes.io/docs/concepts/configuration/secret/

---

## Why These Concepts Matter for Distributed Systems

### For Raft (and similar consensus algorithms):

1. **StatefulSet** provides:
   - Stable network identities (raft-0, raft-1, raft-2)
   - Ordered deployment (important for bootstrap)
   - Persistent storage per node (Raft logs must survive restarts)

2. **Headless Service** provides:
   - Direct pod-to-pod communication (no load balancing)
   - DNS entries for each pod (raft-0.raft-headless.default.svc.cluster.local)
   - Stable endpoints for peer discovery

3. **PersistentVolumes** provide:
   - Durable storage for Raft logs
   - Survives pod restarts and rescheduling
   - Essential for Raft correctness (can't lose committed logs)

4. **ConfigMap** provides:
   - Cluster configuration (peer addresses, timeouts)
   - Can be updated without rebuilding images

5. **Regular Service (ClusterIP)** provides:
   - Stable endpoint for clients to reach the cluster
   - Load balancing (clients don't need to know which node is leader)

### Key Insight

Kubernetes resources are building blocks. For distributed systems like Raft:
- **StatefulSet** = Stable nodes
- **Headless Service** = Peer discovery
- **Regular Service** = Client access
- **PersistentVolume** = Durable state
- **ConfigMap** = Cluster config

In the next section, we'll create the actual YAML manifests for a Raft cluster using these concepts.

**Continue to**: [Part 8: Kubernetes Manifests](08-kubernetes-manifests.md)
