#!/usr/bin/env bash

# Titan Agent System Troubleshooting Script
# Helps diagnose common issues with deployment and operation

set -e

info() { echo -e "\033[1;34m[INFO]\033[0m $1"; }
warn() { echo -e "\033[1;33m[WARN]\033[0m $1"; }
check() { echo -e "\033[1;32m[✓]\033[0m $1"; }
error() { echo -e "\033[1;31m[✗]\033[0m $1"; }

echo "═══════════════════════════════════════════════════════════════"
echo "Titan Agent System - Deployment Troubleshooting"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Section 1: System Requirements
echo "📋 Checking System Requirements..."
echo ""

if command -v kubectl &> /dev/null; then
    check "kubectl installed: $(kubectl version --client --short 2>/dev/null | head -1)"
else
    error "kubectl not installed. Install from: https://kubernetes.io/docs/tasks/tools/"
fi

if command -v cargo &> /dev/null; then
    check "Rust installed: $(rustc --version)"
else
    error "Rust not installed. Install from: https://rustup.rs/"
fi

if command -v docker &> /dev/null; then
    check "Docker installed: $(docker --version)"
else
    warn "Docker not installed (optional, needed for container builds)"
fi

if command -v psql &> /dev/null; then
    check "PostgreSQL client installed"
else
    warn "PostgreSQL client not installed (optional, for database debugging)"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "🔍 Checking Kubernetes Cluster Status"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if kubectl cluster-info &> /dev/null; then
    check "Kubernetes cluster is accessible"
    echo "Cluster info:"
    kubectl cluster-info | grep -v "To further debug"
else
    error "Cannot connect to Kubernetes cluster"
    error "Solutions:"
    echo "  1. Start your cluster: minikube start (or docker desktop)"
    echo "  2. Check kubeconfig: export KUBECONFIG=path/to/config"
    echo "  3. Verify permissions: kubectl auth can-i create pods"
    exit 1
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "📦 Checking Titan Agent Namespace & Pods"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if kubectl get namespace titan-agents &> /dev/null; then
    check "Namespace 'titan-agents' exists"
    
    echo ""
    info "Pod Status:"
    kubectl get pods -n titan-agents --no-headers 2>/dev/null || echo "No pods found"
    
    echo ""
    info "Deployments:"
    kubectl get deployments -n titan-agents --no-headers 2>/dev/null || echo "No deployments found"
    
    echo ""
    info "Services:"
    kubectl get svc -n titan-agents --no-headers 2>/dev/null || echo "No services found"
else
    warn "Namespace 'titan-agents' not found"
    warn "This is expected if you haven't run activation yet"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "🌐 Checking Network Connectivity"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if kubectl get svc titan-agent-bridge-svc -n titan-agents &> /dev/null 2>&1; then
    info "Checking gRPC Bridge Service..."
    GRPC_IP=$(kubectl get svc titan-agent-bridge-svc -n titan-agents -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "pending")
    
    if [ "$GRPC_IP" != "pending" ] && [ -n "$GRPC_IP" ]; then
        check "gRPC endpoint is accessible at: $GRPC_IP:50051"
    else
        warn "LoadBalancer IP is pending (this is normal on minikube/docker-desktop)"
        info "Use port-forward instead: kubectl port-forward svc/titan-agent-bridge-svc 50051:50051 -n titan-agents"
    fi
else
    warn "gRPC Bridge service not found (deployment may not be complete)"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "💾 Checking Database Connectivity"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if kubectl get pod -n titan-agents -l app=ledger-db &> /dev/null 2>&1; then
    check "PostgreSQL pod found"
    
    DB_POD=$(kubectl get pod -n titan-agents -l app=ledger-db -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)
    
    if [ -n "$DB_POD" ]; then
        info "Testing database connection..."
        
        if kubectl exec -n titan-agents "$DB_POD" -- psql -U postgres -c "SELECT 1" &> /dev/null; then
            check "Database is responsive"
        else
            warn "Database is not responding to queries yet"
            warn "Wait for pod to be ready: kubectl logs $DB_POD -n titan-agents"
        fi
    fi
else
    warn "PostgreSQL pod not found"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "📊 Checking Prometheus & Metrics"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if kubectl get crd prometheusrules.monitoring.coreos.com &> /dev/null 2>&1; then
    check "Prometheus CRDs are installed"
    
    if kubectl get prometheusrule titan-agent-alerts -n titan-agents &> /dev/null 2>&1; then
        check "Prometheus alert rules configured"
    else
        warn "Alert rules not yet deployed"
    fi
else
    warn "Prometheus operator not installed (optional, for advanced monitoring)"
fi

echo ""
echo "═════════���═════════════════════════════════════════════════════"
echo "🔐 Checking Secrets"
echo "═══════════════════════════════════════════════════════════════"
echo ""

if kubectl get secret titan-crypto-vault -n titan-agents &> /dev/null 2>&1; then
    check "EVM private key secret is configured"
else
    warn "EVM private key secret not found"
    info "Create it with: kubectl create secret generic titan-crypto-vault \\"
    echo "  --namespace=titan-agents \\"
    echo "  --from-literal=evm-private-key='0xYOUR_KEY'"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "🔧 Quick Fixes"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "If you're having issues, try these commands:"
echo ""
echo "1. View agent pod logs:"
echo "   kubectl logs -n titan-agents -l app=titan-agent --tail=50"
echo ""
echo "2. Describe agent pod:"
echo "   kubectl describe pod -n titan-agents -l app=titan-agent"
echo ""
echo "3. Check service endpoints:"
echo "   kubectl get endpoints -n titan-agents"
echo ""
echo "4. Port-forward for local testing:"
echo "   kubectl port-forward svc/titan-agent-bridge-svc 50051:50051 -n titan-agents"
echo ""
echo "5. View database logs:"
echo "   kubectl logs -n titan-agents -l app=ledger-db --tail=20"
echo ""
echo "6. Re-deploy everything:"
echo "   ./src/wipe_all.sh"
echo "   ./src/activate_all.sh"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✅ Troubleshooting Complete"
echo "═══════════════════════════════════════════════════════════════"
