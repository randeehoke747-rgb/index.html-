#!/usr/bin/env bash
set -e

info() { echo -e "\033[1;34m[INFO]\033[0m $1"; }
warn() { echo -e "\033[1;33m[WARN]\033[0m $1"; }
check() { echo -e "\033[1;32m[OK]\033[0m $1"; }

info "⚡ Starting Monolithic Titan Agent System Activation Pipeline"

# -----------------------------------------------------------
# NETWORK IP DETECTION & FIREWALL CONFIGURATION
# -----------------------------------------------------------
info "Scanning network interfaces..."
DETECTED_IP=$(hostname -I | awk '{print $1}')

# Clean local firewall paths
sudo apt-get update && sudo apt-get install -y ufw
sudo ufw allow 50051/tcp comment 'Titan iPhone gRPC'
sudo ufw allow 8080/tcp comment 'Titan Gateway Metrics'
sudo ufw allow 3000/tcp comment 'Titan Grafana UI'
sudo ufw allow 5432/tcp comment 'Titan PostgreSQL'
sudo ufw reload

echo -e "\033[1;35m┌─────────────────────────────────────────────────────────┐"
echo -e "\033[1;35m│ IPHONE 14 AUTOCONNECT COORDINATES"
echo -e "\033[1;35m├─────────────────────────────────────────────────────────┤"
echo -e "\033[1;35m│ Full gRPC Endpoint : \033[1;32m$DETECTED_IP:50051"
echo -e "\033[1;35m└─────────────────────────────────────────────────────────┘"

# -----------------------------------------------------------
# RESOURCE INITIALIZATION
# -----------------------------------------------------------
info "Creating Kubernetes namespace and secrets..."
kubectl create namespace titan-agents --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic titan-crypto-vault \
  --namespace=titan-agents \
  --from-literal=evm-private-key="0xYOUR_SECRET_PRIVATE_KEY" \
  --dry-run=client -o yaml | kubectl apply -f -

info "Building Rust application..."
cargo build --release

info "Deploying PostgreSQL ledger..."
kubectl apply -f ./titan-system/k8s/database-ledger.yaml

info "Deploying Titan Agent system..."
kubectl apply -k ./titan-system/k8s/

info "Applying Prometheus alerts..."
kubectl apply -f ./titan-system/k8s/prometheus-alerts.yaml

info "Deploying Grafana dashboard..."
kubectl apply -f ./titan-system/k8s/grafana-dashboard.yaml

info "Awaiting pod readiness verification..."
kubectl rollout status deployment/titan-ledger-db -n titan-agents
kubectl rollout status deployment/titan-agent-core -n titan-agents

check "✅ SYSTEM ARMED AND READY FOR ACTION NOW."

echo ""
echo "Next steps:"
echo "  1. Configure EVM_PRIVATE_KEY in secrets"
echo "  2. Port-forward: kubectl port-forward svc/titan-agent-bridge-svc 50051:50051 -n titan-agents"
echo "  3. Connect mobile dashboard to $DETECTED_IP:50051"
echo ""

RUN cargo build --release --workspace
COPY --from=builder /app/target/release/titan-browser-launcher .
COPY --from=builder /app/target/release/titan-node .
COPY --from=builder /app/web ./web
CMD ["./titan-browser-launcher"]