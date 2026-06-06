#!/usr/bin/env bash

# Titan Agent System - Health Check
# Monitors system health and provides real-time status

NAMESPACE="titan-agents"
INTERVAL=5

health() { echo -e "\033[1;32m[HEALTHY]\033[0m $1"; }
unhealthy() { echo -e "\033[1;31m[UNHEALTHY]\033[0m $1"; }
warning() { echo -e "\033[1;33m[WARNING]\033[0m $1"; }
info() { echo -e "\033[1;34m[INFO]\033[0m $1"; }

echo "═════════════════════════════��═════════════════════════════════"
echo "Titan Agent System - Health Monitor"
echo "═══════════════════════════════════════════════════════════════"
echo "Press Ctrl+C to stop monitoring"
echo ""

while true; do
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] Health Check"
    echo "───────────────────────────────────────────────────────────────"
    
    # Check pod status
    AGENT_RUNNING=$(kubectl get pod -n $NAMESPACE -l app=titan-agent --field-selector=status.phase=Running 2>/dev/null | wc -l)
    AGENT_TOTAL=$(kubectl get pod -n $NAMESPACE -l app=titan-agent 2>/dev/null | wc -l)
    
    if [ "$AGENT_RUNNING" -gt 1 ]; then
        health "Agent pods: $((AGENT_RUNNING - 1))/2 ready"
    else
        unhealthy "Agent pods: not running"
    fi
    
    # Check database
    DB_RUNNING=$(kubectl get pod -n $NAMESPACE -l app=ledger-db --field-selector=status.phase=Running 2>/dev/null | wc -l)
    if [ "$DB_RUNNING" -gt 1 ]; then
        health "Database pod: Running"
    else
        unhealthy "Database pod: Not running"
    fi
    
    # Check memory usage
    if command -v kubectl &> /dev/null; then
        MEM=$(kubectl top pod -n $NAMESPACE -l app=titan-agent --no-headers 2>/dev/null | awk '{sum+=$2} END {print sum}' || echo 0)
        if [ "$MEM" -gt 0 ]; then
            info "Memory usage: ${MEM}Mi"
        fi
    fi
    
    # Check service
    SVC_ENDPOINTS=$(kubectl get endpoints titan-agent-bridge-svc -n $NAMESPACE -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null | wc -w)
    if [ "$SVC_ENDPOINTS" -gt 0 ]; then
        health "Service endpoints: $SVC_ENDPOINTS active"
    else
        warning "Service endpoints: 0 active"
    fi
    
    # Check logs for errors
    ERROR_COUNT=$(kubectl logs -n $NAMESPACE -l app=titan-agent --tail=100 2>/dev/null | grep -ic "error" || echo 0)
    if [ "$ERROR_COUNT" -gt 0 ]; then
        warning "Recent errors found: $ERROR_COUNT"
    else
        health "No recent errors in logs"
    fi
    
    echo ""
    echo "Waiting ${INTERVAL}s until next check..."
    sleep $INTERVAL
    clear
done
