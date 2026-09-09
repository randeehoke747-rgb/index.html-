-- Titan Security Monitoring System - Database Schema

-- Monitored Sites Table
CREATE TABLE IF NOT EXISTS monitored_sites (
    id SERIAL PRIMARY KEY,
    url VARCHAR(255) NOT NULL UNIQUE,
    wallet_address VARCHAR(42),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Client Wallets Table
CREATE TABLE IF NOT EXISTS client_wallets (
    id SERIAL PRIMARY KEY,
    client_id VARCHAR(255) NOT NULL,
    address VARCHAR(42) NOT NULL,
    is_active BOOLEAN DEFAULT true,
    is_authorized BOOLEAN DEFAULT false,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(client_id, address)
);

-- Client Authorizations Table
CREATE TABLE IF NOT EXISTS client_authorizations (
    id SERIAL PRIMARY KEY,
    client_id VARCHAR(255) NOT NULL,
    wallet_address VARCHAR(42) NOT NULL,
    authorization_signature VARCHAR(255),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    UNIQUE(client_id, wallet_address)
);

-- Wallet Balance History Table
CREATE TABLE IF NOT EXISTS wallet_balance_history (
    id SERIAL PRIMARY KEY,
    wallet_address VARCHAR(42) NOT NULL,
    balance NUMERIC(20, 6) NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Monitoring Events Table
CREATE TABLE IF NOT EXISTS monitoring_events (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    event_data TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Transaction Events Table
CREATE TABLE IF NOT EXISTS transaction_events (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Fund Redemption History Table
CREATE TABLE IF NOT EXISTS redemption_history (
    id SERIAL PRIMARY KEY,
    client_id VARCHAR(255) NOT NULL,
    from_wallet VARCHAR(42) NOT NULL,
    to_wallet VARCHAR(42) NOT NULL,
    amount NUMERIC(20, 6) NOT NULL,
    transaction_hash VARCHAR(255),
    status VARCHAR(50) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    confirmed_at TIMESTAMP
);

-- Create Indexes for Performance
CREATE INDEX idx_monitored_sites_active ON monitored_sites(is_active);
CREATE INDEX idx_client_wallets_active ON client_wallets(is_active, is_authorized);
CREATE INDEX idx_wallet_balance_address_time ON wallet_balance_history(wallet_address, timestamp);
CREATE INDEX idx_monitoring_events_type_time ON monitoring_events(event_type, timestamp);
CREATE INDEX idx_transaction_events_type_time ON transaction_events(event_type, timestamp);
CREATE INDEX idx_redemption_status ON redemption_history(status, created_at);
