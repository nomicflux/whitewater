#!/bin/sh 
# setup-kind.sh 

CLUSTER_NAME="cka-lab"

kind create cluster --name $CLUSTER_NAME --config=- <<EOF
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
  - role: control-plane
  - role: worker
  - role: worker
EOF

echo "✅ Kind cluster is ready!"
kubectl cluster-info --context kind-$CLUSTER_NAME
