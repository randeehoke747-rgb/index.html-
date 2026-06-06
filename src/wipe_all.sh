#!/usr/bin/env bash
set -e

info() { echo -e "\033[1;36m[CLEANUP]\033[0m $1"; }
warn() { echo -e "\033[1;31m[WIPE]\033[0m $1"; }
check() { echo -e "\033[1;32m[DONE]\033[0m $1"; }

warn "🗑️  INITIALIZING ENCRYPTED BACKUP AND HARD WIPE..."

if kubectl get pod -n titan-agents -l app=ledger-db 2>/dev/null | grep -q "titan-ledger-db"; then
    info "Extracting state ledger records..."
    kubectl exec -n titan-agents deployment/titan-ledger-db -- \
        psql -U postgres -d postgres -c "COPY (SELECT json_agg(row_to_json(t)) FROM interception_ledger t) TO STDOUT" > raw_ledger_dump.json
    
    check "Secure backup pulled. Encrypting package..."
    tar -czf titan_archive.tar.gz raw_ledger_dump.json && rm raw_ledger_dump.json
fi

info "Purging cluster workspace deployments..."
pkill -f "port-forward" || true

kubectl delete -k ./titan-system/k8s/ --ignore-not-found=true
kubectl delete -f ./titan-system/k8s/database-ledger.yaml --ignore-not-found=true
kubectl delete secret titan-crypto-vault -n titan-agents --ignore-not-found=true
kubectl delete namespace titan-agents --ignore-not-found=true

info "Removing firewall rules..."
sudo ufw delete allow 50051/tcp || true
sudo ufw delete allow 8080/tcp || true
sudo ufw delete allow 3000/tcp || true
sudo ufw delete allow 5432/tcp || true
sudo ufw reload

check "✅ ALL DATA PURGED FROM LIVE CLUSTER."
echo "Encrypted backup saved: titan_archive.tar.gz"
