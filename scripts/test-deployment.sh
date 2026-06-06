#!/usr/bin/env bash

# Titan Agent System - Deployment Test Suite
# Validates that all components are working correctly

set -e

SUCCESS=0
FAILURES=0

test_pass() { echo -e "\033[1;32m[PASS]\033[0m $1"; ((SUCCESS++)); }
test_fail() { echo -e "\033[1;31m[FAIL]\033[0m $1"; ((FAILURES++)); }
test_info() { echo -e "\033[1;34m[TEST]\033[0m $1"; }

echo "═══════════════════════════════════════════════════════════════"
echo "Titan Agent System - Deployment Test Suite"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Test 1: Namespace exists
test_info "Checking namespace existence..."
if kubectl get namespace titan-agents &> /dev/null; then
    test_pass "Namespace 'titan-agents' exists"
else
    test_fail "Namespace 'titan-agents' does not exist"
fi

# Test 2: Deployment exists and is ready
test_info "Checking agent deployment..."
if kubectl get deployment titan-agent-core -n titan-agents &> /dev/null; then
    READY=$(kubectl get deployment titan-agent-core -n titan-agents -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo 0)
    DESIRED=$(kubectl get deployment titan-agent-core -n titan-agents -o jsonpath='{.spec.replicas}' 2>/dev/null || echo 0)
    
    if [ "$READY" -eq "$DESIRED" ] && [ "$DESIRED" -gt 0 ]; then
        test_pass "Agent deployment ready ($READY/$DESIRED replicas)"
    else
        test_fail "Agent deployment not ready ($READY/$DESIRED replicas)"
    fi
else
    test_fail "Agent deployment does not exist"
fi

# Test 3: Database pod exists
test_info "Checking database pod..."
if kubectl get pod -n titan-agents -l app=ledger-db &> /dev/null 2>&1; then
    DB_STATUS=$(kubectl get pod -n titan-agents -l app=ledger-db -o jsonpath='{.items[0].status.phase}' 2>/dev/null)
    if [ "$DB_STATUS" = "Running" ]; then
        test_pass "Database pod is running"
    else
        test_fail "Database pod status: $DB_STATUS"
    fi
else
    test_fail "Database pod not found"
fi

# Test 4: gRPC service exists
test_info "Checking gRPC service..."
if kubectl get svc titan-agent-bridge-svc -n titan-agents &> /dev/null; then
    test_pass "gRPC service exists"
else
    test_fail "gRPC service not found"
fi

# Test 5: Database service exists
test_info "Checking database service..."
if kubectl get svc titan-ledger-db -n titan-agents &> /dev/null; then
    test_pass "Database service exists"
else
    test_fail "Database service not found"
fi

# Test 6: Secrets configured
test_info "Checking secrets..."
if kubectl get secret titan-crypto-vault -n titan-agents &> /dev/null 2>&1; then
    test_pass "Crypto vault secret exists"
else
    test_fail "Crypto vault secret not found"
fi

# Test 7: Check pod resource limits
test_info "Checking pod resource allocation..."
AGENT_POD=$(kubectl get pod -n titan-agents -l app=titan-agent -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)
if [ -n "$AGENT_POD" ]; then
    CPU=$(kubectl get pod "$AGENT_POD" -n titan-agents -o jsonpath='{.spec.containers[0].resources.limits.cpu}' 2>/dev/null)
    MEM=$(kubectl get pod "$AGENT_POD" -n titan-agents -o jsonpath='{.spec.containers[0].resources.limits.memory}' 2>/dev/null)
    if [ -n "$CPU" ] && [ -n "$MEM" ]; then
        test_pass "Resource limits set: CPU=$CPU, Memory=$MEM"
    else
        test_fail "Resource limits not configured"
    fi
else
    test_fail "Cannot check pod resources (no pods found)"
fi

# Test 8: Check service endpoints
test_info "Checking service endpoints..."
ENDPOINTS=$(kubectl get endpoints titan-agent-bridge-svc -n titan-agents -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null | wc -w)
if [ "$ENDPOINTS" -gt 0 ]; then
    test_pass "Service endpoints configured ($ENDPOINTS backends)"
else
    test_fail "No service endpoints available"
fi

# Test 9: Network policy check
test_info "Checking network policies..."
if kubectl get networkpolicy -n titan-agents &> /dev/null 2>&1; then
    POLICIES=$(kubectl get networkpolicy -n titan-agents --no-headers 2>/dev/null | wc -l)
    if [ "$POLICIES" -gt 0 ]; then
        test_pass "Network policies configured ($POLICIES policies)"
    else
        test_fail "No network policies found"
    fi
else
    test_fail "Network policies not supported or configured"
fi

# Test 10: Resource quota check
test_info "Checking resource quotas..."
if kubectl get resourcequota -n titan-agents &> /dev/null 2>&1; then
    test_pass "Resource quota exists"
else
    test_fail "No resource quota configured (optional)"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "Test Results"
echo "═══════════════════════════════════════════════════════════════"
echo "Passed: $SUCCESS"
echo "Failed: $FAILURES"
echo ""

if [ $FAILURES -eq 0 ]; then
    echo -e "\033[1;32m✅ All tests passed! System is ready.\033[0m"
    exit 0
else
    echo -e "\033[1;31m❌ $FAILURES test(s) failed. Check deployment.\033[0m"
    exit 1
fi
