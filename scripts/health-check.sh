#!/usr/bin/env bash

# Titan Agent System - Production Health Monitor
# Real-time observability for Kubernetes deployment

NAMESPACE="titan-agents"
INTERVAL=5

health()   { echo -e "\033[1;32m[HEALTHY]\033[0m $1"; }
unhealthy(){ echo -e "\033[1;31m[UNHEALTHY]\033[0m $1"; }
warning()  { echo -e "\033[1;33m[WARNING]\033[0m $1"; }
info()     { echo -e "\033[1;34m[INFO]\033[0m $1"; }

echo "═══════════════════════════════════════════════════════════════"
echo "Titan Agent System - Live Health Monitor"
echo "Namespace: $NAMESPACE"
echo "Interval: ${INTERVAL}s"
echo "═══════════════════════════════════════════════════════════════"
echo ""

while true; do

    TS=$(date +'%Y-%m-%d %H:%M:%S')
    echo "[$TS] System Status"

    echo "───────────────────────────────────────────────────────────────"

    # ----------------------------
    # 1. Agent Deployment Health
    # ----------------------------
    DESIRED=$(kubectl get deployment titan-agent-core -n "$NAMESPACE" -o jsonpath='{.spec.replicas}' 2>/dev/null || echo 0)
    READY=$(kubectl get deployment titan-agent-core -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo 0)

    if [ "$DESIRED" -eq 0 ]; then
        warning "Agent deployment not found or not configured"
    elif [ "$READY" -eq "$DESIRED" ]; then
        health "Agent core: $READY/$DESIRED ready"
    else
        unhealthy "Agent core: $READY/$DESIRED ready"
    fi

    # ----------------------------
    # 2. Database Health
    # ----------------------------
    DB_READY=$(kubectl get pod -n "$NAMESPACE" -l app=ledger-db \
        -o jsonpath='{.items[*].status.phase}' 2>/dev/null | grep -c Running || true)

    if [ "$DB_READY" -ge 1 ]; then
        health "Ledger DB: Running"
    else
        unhealthy "Ledger DB: Not running"
    fi

    # ----------------------------
    # 3. Service Endpoint Health
    # ----------------------------
    ENDPOINTS=$(kubectl get endpoints titan-agent-bridge-svc -n "$NAMESPACE" \
        -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null | wc -w | tr -d ' ')

    if [ "$ENDPOINTS" -gt 0 ]; then
        health "Bridge service endpoints: $ENDPOINTS"
    else
        warning "Bridge service has no active endpoints"
    fi

    # ----------------------------
    # 4. Pod Crash Detection
    # ----------------------------
    CRASH_COUNT=$(kubectl get pods -n "$NAMESPACE" \
        --field-selector=status.phase=Failed 2>/dev/null | wc -l | tr -d ' ')

    if [ "$CRASH_COUNT" -gt 0 ]; then
        unhealthy "Failed pods detected: $CRASH_COUNT"
    else
        health "No failed pods"
    fi

    # ----------------------------
    # 5. Restart Pressure (real signal)
    # ----------------------------
    RESTARTS=$(kubectl get pods -n "$NAMESPACE" \
        -o jsonpath='{.items[*].status.containerStatuses[*].restartCount}' 2>/dev/null \
        | awk '{for(i=1;i<=NF;i++) s+=$i} END {print s+0}')

    if [ "$RESTARTS" -gt 5 ]; then
        warning "High restart count: $RESTARTS"
    else
        info "Container restarts: $RESTARTS"
    fi

    # ----------------------------
    # 6. Memory/CPU (only if metrics-server exists)
    # ----------------------------
    if kubectl top pod -n "$NAMESPACE" &>/dev/null; then
        CPU=$(kubectl top pod -n "$NAMESPACE" --no-headers 2>/dev/null | awk '{sum+=$2} END {print sum}')
        MEM=$(kubectl top pod -n "$NAMESPACE" --no-headers 2>/dev/null | awk '{sum+=$3} END {print sum}')

        [ -n "$CPU" ] && info "CPU usage: ${CPU}m"
        [ -n "$MEM" ] && info "Memory usage: ${MEM}"
    else
        warning "Metrics server not available"
    fi

    echo ""
    echo "Next update in ${INTERVAL}s..."
    sleep "$INTERVAL"
    echo ""
done

