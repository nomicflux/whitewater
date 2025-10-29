#!/bin/bash

kind delete clusters whitewater-cluster
kind create cluster --config kind-cluster.yaml
kind load docker-image whitewater:2 --name whitewater-cluster

kubectl apply -f ingress-controller.yaml
#kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.10.1/deploy/static/provider/kind/deploy.yaml
#kubectl apply -f ingress-nginx.yaml
kubectl wait --namespace ingress-nginx \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/component=controller \
  --timeout=90s
kubectl apply -f deploy.yaml
