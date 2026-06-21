#!/usr/bin/env bash

# Titan Agent System - Production Troubleshooting Script
# Real cluster diagnostics (safe, non-failing, timeout-protected)

set +e

NAMESPACE="titan-agents"
TIMEOUT=5

info()  { echo -e "\033[1;34m[INFO]\033[0m $1"; }
warn()  { echo -e "\033[1;33m[WARN]\033[0m $1"; }
ok()    { echo -e "\033[1;32m[✓]\033[0m $1"; }
fail()  { echo -e "\033[1;31m[✗]\033[0m $1"; }

run() {
    timeout $TIMEOUT bash -c "$1" 2>/dev/null
}

echo "═══════════════════════════════════════════════════════════════"
echo "Titan Agent System - Production Troubleshooting"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# -------------------------------------------------------
# 1. CLI TOOLS CHECK
# -------------------------------------------------------
info "Checking system tools..."

command -v kubectl >/dev/null && ok "kubectl installed" || fail "kubectl missing"

command -v docker >/dev/null && ok "docker installed" || warn "docker missing (optional)"

command -v psql >/dev/null && ok "psql installed" || warn "psql missing (optional DB debugging)"

echo ""

# -------------------------------------------------------
# 2. CLUSTER ACCESS
# -------------------------------------------------------
info "Checking Kubernetes cluster..."

if run "kubectl cluster-info"; then
    ok "Cluster accessible"
else
    fail "Cluster not reachable"
    echo ""
    echo "Fix:"
    echo "  - minikube start"
    echo "  - docker desktop Kubernetes enabled"
    echo "  - kubectl config current-context"
    exit 1
fi

echo ""

# -------------------------------------------------------
# 3. NAMESPACE CHECK
# -------------------------------------------------------
info "Checking namespace..."

if kubectl get ns "$NAMESPACE" >/dev/null 2>&1; then
    ok "Namespace exists: $NAMESPACE"
else
    warn "Namespace missing (deployment not run yet)"
    echo "Fix: ./activate_all.sh"
    exit 0
fi

echo ""

# -------------------------------------------------------
# 4. POD STATUS ANALYSIS
# -------------------------------------------------------
info "Analyzing pods..."

PODS=$(kubectl get pods -n "$NAMESPACE" --no-headers 2>/dev/null)

if [ -z "$PODS" ]; then
    warn "No pods found in namespace"
else
    echo "$PODS"

    RUNNING=$(echo "$PODS" | grep Running | wc -l | tr -d ' ')
    PENDING=$(echo "$PODS" | grep Pending | wc -l | tr -d ' ')
    FAILED=$(echo "$PODS" | grep -E "Error|CrashLoopBackOff|Failed" | wc -l | tr -d ' ')

    [ "$RUNNING" -gt 0 ] && ok "$RUNNING pods running"
    [ "$PENDING" -gt 0 ] && warn "$PENDING pods pending"
    [ "$FAILED" -gt 0 ] && fail "$FAILED pods failing"
fi

echo ""

# -------------------------------------------------------
# 5. DEPLOYMENT HEALTH
# -------------------------------------------------------
info "Checking deployments..."

DEPLOYMENTS=$(kubectl get deploy -n "$NAMESPACE" --no-headers 2>/dev/null)

if [ -n "$DEPLOYMENTS" ]; then
    echo "$DEPLOYMENTS"
else
    warn "No deployments found"
fi

echo ""

# -------------------------------------------------------
# 6. SERVICES
# -------------------------------------------------------
info "Checking services..."

SERVICES=$(kubectl get svc -n "$NAMESPACE" --no-headers 2>/dev/null)

if [ -n "$SERVICES" ]; then
    echo "$SERVICES"
    ok "Services loaded"
else
    warn "No services found"
fi

echo ""

# -------------------------------------------------------
# 7. ENDPOINT DIAGNOSIS
# -------------------------------------------------------
info "Checking service endpoints..."

ENDPOINTS=$(kubectl get endpoints -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l | tr -d ' ')

if [ "$ENDPOINTS" -gt 0 ]; then
    ok "Endpoints active ($ENDPOINTS)"
else
    warn "No endpoints connected (pods not ready or selector mismatch)"
fi

echo ""

# -------------------------------------------------------
# 8. DATABASE CHECK (SAFE)
# -------------------------------------------------------
info "Checking database..."

DB_POD=$(kubectl get pod -n "$NAMESPACE" -l app=ledger-db -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)

if [ -n "$DB_POD" ]; then
    ok "DB pod exists: $DB_POD"

    PHASE=$(kubectl get pod "$DB_POD" -n "$NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null)

    if [ "$PHASE" = "Running" ]; then
        ok "Database running"
    else
        warn "Database state: $PHASE"
    fi
else
    warn "Database pod not found"
fi

echo ""

# -------------------------------------------------------
# 9. LOADBALANCER / ACCESS POINTS
# -------------------------------------------------------
info "Checking external access..."

SVC_TYPE=$(kubectl get svc titan-agent-bridge-svc -n "$NAMESPACE" -o jsonpath='{.spec.type}' 2>/dev/null)

if [ "$SVC_TYPE" = "LoadBalancer" ]; then
    IP=$(kubectl get svc titan-agent-bridge-svc -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null)

    if [ -n "$IP" ]; then
        ok "External IP: $IP"
    else
        warn "LoadBalancer pending (normal on local clusters)"
        info "Use port-forward instead:"
        echo "kubectl port-forward svc/titan-agent-bridge-svc 50051:50051 -n $NAMESPACE"
    fi
else
    warn "Service is not LoadBalancer (likely ClusterIP)"
fi

echo ""

# -------------------------------------------------------
# 10. LOG QUICK DIAGNOSIS
# -------------------------------------------------------
info "Recent agent logs (errors only):"

kubectl logs -n "$NAMESPACE" -l app=titan-agent --tail=30 2>/dev/null | grep -i error || echo "No errors found"

echo ""

# -------------------------------------------------------
# FINAL SUMMARY
# -------------------------------------------------------
echo "═══════════════════════════════════════════════════════════════"
echo "Troubleshooting Complete"
echo "═══════════════════════════════════════════════════════════════"

info "Next steps if issues persist:"
echo "  kubectl describe pod -n $NAMESPACE -l app=titan-agent"
echo "  kubectl get events -n $NAMESPACE --sort-by=.metadata.creationTimestamp"
echo "  kubectl logs -n $NAMESPACE -l app=titan-agent"
echo ""