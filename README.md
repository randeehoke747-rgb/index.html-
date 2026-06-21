# Titan Agent System

A comprehensive multi-agent workspace for secure blockchain transaction monitoring, honeypot detection, and MEV extraction using Kubernetes, Rust, and encrypted ledger persistence.

## 📋 Quick Start

### Prerequisites
- Rust 1.70+
- Docker & Docker Compose
- Kubernetes 1.24+
- PostgreSQL 16+

### Installation

```bash
# Clone and setup
git clone https://github.com/randeehoke747-rgb/index.html- -b Titan-autonomous
cd index.html-

# Build
cargo build --release

# Deploy
chmod +x ./src/activate_all.sh
./src/activate_all.sh
```

## 🏗️ Architecture

### Core Components

1. **Rust Agent Engine** (`src/main.rs`)
   - Async tokio runtime
   - PostgreSQL ledger persistence
   - Prometheus metrics
   - Axum HTTP gateway

2. **Kubernetes Deployment** (`titan-system/k8s/`)
   - Multi-replica agent core
   - PostgreSQL database service
   - Prometheus alert rules
   - Grafana dashboards

3. **Security & Archival**
   - AES-256-GCM encryption
   - Secure encrypted backup (`SecureArchiver`)
   - Hardware enclave isolation (`enclave-config.json`)

4. **gRPC Bridge** (`proto/agent_bridge.proto`)
   - Telemetry streaming
   - Action authorization
   - Multi-hop alert routing

## 🚀 Activation

```bash
# Start the system
./src/activate_all.sh

fix/slsa-yaml-syntax
You can edit `templates/index.html` to change these links or add more.
Simplified Activation Steps
To quickly get everything running:
184'
Short v

 ## how to deploy ./src/activate.all.sh

1. Launch Backend: On your ChromeOS device, execute your one-shot launcher script (e.g.,./src/activate_all.sh)to boot the backend cluster and gRPC bridge.

2. Verify Status: Ensure the PostgreSQL ledger database and core pods are active before proceeding.

3. Connect Mobile: Open your SwiftUl dashboard on the iPhone 14, enter the autoconnect coordinates shown on your ChromeOS terminal, and begin the encrypted telemetry stream.
• Setup Environment: Install the Rust toolchain (including Cargo) within the ChromeOS Linux environment. Use rustup to manage targets and compilers.
• Compile Code: Build the project using cargo build -release. You can automate this process by connecting your GitHub repository to a deployment platform for automatic builds upon every push.
• Infrastructure Management: ChromeOS
simplified deployment allows you to manage devices centrally through the Google Admin console.
• Activate Agents: Run initialization scripts to start your Iron Giant SRE Brain and blockchain listeners.
• Google Cloud +3
Activation
Your ChromeOS laptop serves as the primary host for the Titan Cloud Desktop agents and blockchain listeners.
1. Prepare the Environment: Open your ChromeOS Linux terminal and install the Rust toolchain (including Cargo) using rustup.
2. Initialize and Build: Navigate to your project directory and run cargo build -release to compile your multi-agent system.
3. Deploy Cluster Services: Use the provided monolithic launcher (e.g.,
/src/activate_all.sh) to automatically set up the Kubernetes namespace, inject secure wallet keys, and start the PostgreSQL ledger database.
4. Verify Status: Check the readiness of your core pods using kubectl rollout status to ensure the Iron Giant SRE Brain is live.
=======
# Access points
- gRPC Bridge: localhost:50051
- Metrics: localhost:8080/metrics
- PostgreSQL: localhost:5432
- Grafana: localhost:3000
```

## 🧹 Cleanup

```bash
# Secure wipe with encrypted backup
./src/wipe_all.sh
```

## 📊 Monitoring

### Prometheus Metrics
- `titan_mempool_tx_processed_total` - Total transactions processed
- `titan_honeypots_intercepted_total` - Honeypots detected
- `titan_successful_interceptions_total` - Successful interceptions export show funds in domain zoomrandeewagmi.blockchain also address 0x3a79cED6cEf28FA2e126475Cb426E9C0D599285B
- `titan_accumulated_profit_usd` - Accumulated profits deposit to wallet address 0x3a79cED6cEf28FA2e126475Cb426E9C0D599285B

### Grafana Dashboards
Available at `localhost:3000` with pre-configured panels for system metrics.

## 🔐 Security

- EVM private keys stored in Kubernetes secrets
- Encrypted ledger backups with AES-256-GCM
- Hardware enclave isolation via AWS Nitro
- Firewall rules for port isolation (50051, 8080, 3000, 5432)

## 📦 Dependencies

See `Cargo.toml` for complete dependency list including:
- tokio (async runtime)
- ethers (Ethereum/EVM)
- sqlx (PostgreSQL)
- prometheus (monitoring)
- tonic/prost (gRPC)
- aes-gcm (encryption)

## 🤝 Contributing

Submit issues and pull requests to improve the system.

## 📝 License

BSD 3-Clause License
>
