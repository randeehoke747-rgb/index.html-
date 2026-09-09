#!/usr/bin/env python3

"""
Titan Security Monitoring System - Transaction Monitor Agent
Autonomous agent that monitors blockchain transactions and redirects compromised funds
"""

import os
import asyncio
import logging
from datetime import datetime
import psycopg2
from psycopg2.extras import RealDictCursor
from web3 import Web3
from eth_account import Account
from decimal import Decimal
import json

# Configure logging
logging.basicConfig(
    level=os.getenv('AGENT_LOG_LEVEL', 'INFO'),
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('TransactionMonitor')

class TitanTransactionMonitor:
    """Autonomous agent for monitoring and redirecting compromised USDC transactions"""

    def __init__(self):
        self.db_url = os.getenv('DATABASE_URL')
        self.ethereum_rpc = os.getenv('ETHEREUM_RPC_URL')
        self.safe_wallet = os.getenv('SAFE_WALLET_ADDRESS')
        self.usdc_contract = os.getenv('USDC_CONTRACT_ADDRESS')
        self.chain_id = int(os.getenv('ETHEREUM_CHAIN_ID', '1'))
        self.monitor_interval = int(os.getenv('TRANSACTION_MONITOR_INTERVAL', '60'))
        
        # Web3 connection
        self.w3 = Web3(Web3.HTTPProvider(self.ethereum_rpc))
        
        if not self.w3.is_connected():
            raise Exception("Failed to connect to Ethereum RPC")
        
        logger.info(f"Titan Transaction Monitor initialized")
        logger.info(f"Safe Wallet: {self.safe_wallet}")
        logger.info(f"USDC Contract: {self.usdc_contract}")
        logger.info(f"Chain ID: {self.chain_id}")

    def get_db_connection(self):
        """Establish PostgreSQL connection"""
        return psycopg2.connect(self.db_url)

    def get_client_wallets(self):
        """Fetch list of client wallets to monitor"""
        try:
            conn = self.get_db_connection()
            with conn.cursor(cursor_factory=RealDictCursor) as cur:
                cur.execute(
                    "SELECT * FROM client_wallets WHERE is_active = true AND is_authorized = true"
                )
                wallets = cur.fetchall()
            conn.close()
            return wallets
        except Exception as e:
            logger.error(f"Database error fetching client wallets: {e}")
            return []

    def verify_client_authorization(self, client_id, wallet_address):
        """Verify client has authorized fund recovery for this wallet"""
        try:
            conn = self.get_db_connection()
            with conn.cursor(cursor_factory=RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT * FROM client_authorizations
                    WHERE client_id = %s AND wallet_address = %s AND is_active = true
                    """,
                    (client_id, wallet_address)
                )
                auth = cur.fetchone()
            conn.close()
            return auth is not None
        except Exception as e:
            logger.error(f"Authorization verification error: {e}")
            return False

    def get_wallet_balance(self, wallet_address):
        """Get USDC balance for wallet"""
        try:
            # USDC ERC-20 ABI (balanceOf function)
            usdc_abi = [
                {
                    "constant": True,
                    "inputs": [{"name": "_owner", "type": "address"}],
                    "name": "balanceOf",
                    "outputs": [{"name": "balance", "type": "uint256"}],
                    "type": "function"
                }
            ]
            
            contract = self.w3.eth.contract(address=self.usdc_contract, abi=usdc_abi)
            balance = contract.functions.balanceOf(wallet_address).call()
            # USDC has 6 decimals
            balance_usdc = balance / (10 ** 6)
            
            return Decimal(str(balance_usdc))
        except Exception as e:
            logger.error(f"Error getting wallet balance: {e}")
            return Decimal(0)

    def monitor_wallet_transactions(self, wallet_address, client_id):
        """Monitor wallet for suspicious transactions"""
        try:
            logger.info(f"Monitoring transactions for wallet: {wallet_address}")
            
            # Get current balance
            balance = self.get_wallet_balance(wallet_address)
            logger.info(f"Wallet balance: {balance} USDC")
            
            # Query database for previous balance
            conn = self.get_db_connection()
            with conn.cursor(cursor_factory=RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT * FROM wallet_balance_history
                    WHERE wallet_address = %s
                    ORDER BY timestamp DESC LIMIT 1
                    """,
                    (wallet_address,)
                )
                last_record = cur.fetchone()
            
            # Check for suspicious balance changes
            if last_record:
                last_balance = Decimal(str(last_record['balance']))
                balance_change = last_balance - balance
                
                if balance_change > 0:  # Balance decreased unexpectedly
                    logger.warning(f"Suspicious balance decrease detected: {balance_change} USDC")
                    self.log_event('suspicious_transaction', {
                        'wallet': wallet_address,
                        'client_id': client_id,
                        'balance_change': str(balance_change),
                        'timestamp': datetime.utcnow().isoformat()
                    })
                    
                    # Verify authorization before redirecting
                    if self.verify_client_authorization(client_id, wallet_address):
                        self.redirect_funds(wallet_address, balance_change, client_id)
            
            # Update balance history
            with conn.cursor() as cur:
                cur.execute(
                    """
                    INSERT INTO wallet_balance_history (wallet_address, balance, timestamp)
                    VALUES (%s, %s, %s)
                    """,
                    (wallet_address, str(balance), datetime.utcnow())
                )
            conn.commit()
            conn.close()
            
        except Exception as e:
            logger.error(f"Error monitoring wallet transactions: {e}")

    def redirect_funds(self, from_wallet, amount, client_id):
        """Execute fund redirection to safe wallet"""
        try:
            logger.info(f"Initiating fund redirection: {amount} USDC from {from_wallet}")
            logger.info(f"Target safe wallet: {self.safe_wallet}")
            
            # Log redirection intent
            self.log_event('fund_redirection', {
                'from_wallet': from_wallet,
                'to_wallet': self.safe_wallet,
                'amount': str(amount),
                'client_id': client_id,
                'status': 'initiated',
                'timestamp': datetime.utcnow().isoformat()
            })
            
            # TODO: Implement actual transaction execution with:
            # - Private key decryption
            # - Gas estimation
            # - Transaction signing
            # - Blockchain submission
            # - Confirmation tracking
            
            logger.info(f"Fund redirection initiated for {amount} USDC")
            
        except Exception as e:
            logger.error(f"Error redirecting funds: {e}")
            self.log_event('redirection_error', {
                'from_wallet': from_wallet,
                'error': str(e),
                'timestamp': datetime.utcnow().isoformat()
            })

    def log_event(self, event_type, event_data):
        """Log transaction event to database"""
        try:
            conn = self.get_db_connection()
            with conn.cursor() as cur:
                cur.execute(
                    """
                    INSERT INTO transaction_events (event_type, event_data, timestamp)
                    VALUES (%s, %s, %s)
                    """,
                    (event_type, json.dumps(event_data), datetime.utcnow())
                )
            conn.commit()
            conn.close()
        except Exception as e:
            logger.error(f"Error logging event: {e}")

    async def run_monitoring_cycle(self):
        """Execute one complete monitoring cycle"""
        logger.info("Starting transaction monitoring cycle...")
        
        wallets = self.get_client_wallets()
        logger.info(f"Monitoring {len(wallets)} client wallets")
        
        for wallet in wallets:
            self.monitor_wallet_transactions(wallet['address'], wallet['client_id'])
        
        logger.info("Transaction monitoring cycle complete")

    async def start(self):
        """Start continuous transaction monitoring"""
        logger.info(f"Titan Transaction Monitor started - checking every {self.monitor_interval}s")
        
        while True:
            try:
                await self.run_monitoring_cycle()
                await asyncio.sleep(self.monitor_interval)
            except KeyboardInterrupt:
                logger.info("Monitoring stopped by user")
                break
            except Exception as e:
                logger.error(f"Monitoring error: {e}")
                await asyncio.sleep(10)  # Brief wait before retry

if __name__ == "__main__":
    monitor = TitanTransactionMonitor()
    asyncio.run(monitor.start())
