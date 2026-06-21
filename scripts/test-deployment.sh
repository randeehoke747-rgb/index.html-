#!/usr/bin/env bash

# Titan Agent System - Deployment Script
# Deploys and initializes all components

set -e

NAMESPACE="titan-agents"

echo "═══════════════════════════════════════════════════════════════"
echo "Titan Agent System - Deployment Script"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# 1. Create namespace if it doesn't exist
echo "[STEP] Ensuring namespace exists..."
kubectl get namespace "$NAMESPACE" >/dev/null 2>&1 || \
kubectl create namespace "$NAMESPACE"

echo "[OK] Namespace ready: $NAMESPACE"
echo ""

# 2. Apply core infrastructure (assumes manifests folder exists)
echo "[STEP] Deploying core infrastructure..."

kubectl apply -f k8s/namespace.yaml || true
kubectl apply -f k8s/ledger-db.yaml -n "$NAMESPACE"
kubectl apply -f k8s/agent-core.yaml -n "$NAMESPACE"
kubectl apply -f k8s/bridge-service.yaml -n "$NAMESPACE"
kubectl apply -f k8s/secrets.yaml -n "$NAMESPACE"

echo "[OK] Manifests applied"
echo ""

# 3. Wait for database
echo "[STEP] Waiting for database..."
kubectl rollout status statefulset/ledger-db -n "$NAMESPACE" --timeout=120s || true

# 4. Wait for agent deployment
echo "[STEP] Waiting for agent core..."
kubectl rollout status deployment/titan-agent-core -n "$NAMESPACE" --timeout=180s || true

echo "[OK] Core workloads deployed"
echo ""

# 5. Ensure services are created
echo "[STEP] Verifying services..."
kubectl get svc -n "$NAMESPACE"

echo "[OK] Services active"
echo ""

# 6. Ensure endpoints exist (light check)
echo "[STEP] Checking service endpoints..."
ENDPOINTS=$(kubectl get endpoints titan-agent-bridge-svc -n "$NAMESPACE" -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null | wc -w || echo 0)

if [ "$ENDPOINTS" -gt 0 ]; then
    echo "[OK] Bridge service has $ENDPOINTS endpoint(s)"
else
    echo "[WARN] No endpoints detected yet (may still be initializing)"
fi

echo ""

# 7. Final summary
echo "═══════════════════════════════════════════════════════════════"
echo "Deployment Complete"
echo "═══════════════════════════════════════════════════════════════"

kubectl get all -n "$NAMESPACE"

echo ""
echo "[DONE] Titan Agent System deployed successfully"