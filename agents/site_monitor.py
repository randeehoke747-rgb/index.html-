#!/usr/bin/env python3

"""
Titan Security Monitoring System - Site Monitor Agent
Autonomous agent that monitors target sites for vulnerabilities and key compromises
"""

import os
import asyncio
import logging
from datetime import datetime
import psycopg2
from psycopg2.extras import RealDictCursor
import requests
from eth_account import Account
from web3 import Web3

# Configure logging
logging.basicConfig(
    level=os.getenv('AGENT_LOG_LEVEL', 'INFO'),
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('SiteMonitor')

class TitanSiteMonitor:
    """Autonomous agent for monitoring target sites and key compromises"""

    def __init__(self):
        self.db_url = os.getenv('DATABASE_URL')
        self.ethereum_rpc = os.getenv('ETHEREUM_RPC_URL')
        self.safe_wallet = os.getenv('SAFE_WALLET_ADDRESS')
        self.usdc_contract = os.getenv('USDC_CONTRACT_ADDRESS')
        self.chain_id = int(os.getenv('ETHEREUM_CHAIN_ID', '1'))
        self.monitor_interval = int(os.getenv('SITE_MONITOR_INTERVAL', '300'))
        self.w3 = Web3(Web3.HTTPProvider(self.ethereum_rpc))
        
        logger.info(f"Titan Site Monitor initialized - Safe Wallet: {self.safe_wallet}")

    def get_db_connection(self):
        """Establish PostgreSQL connection"""
        return psycopg2.connect(self.db_url)

    def get_monitored_sites(self):
        """Fetch list of monitored sites from database"""
        try:
            conn = self.get_db_connection()
            with conn.cursor(cursor_factory=RealDictCursor) as cur:
                cur.execute(
                    "SELECT * FROM monitored_sites WHERE is_active = true"
                )
                sites = cur.fetchall()
            conn.close()
            return sites
        except Exception as e:
            logger.error(f"Database error fetching monitored sites: {e}")
            return []

    def check_site_health(self, site_url, site_id):
        """Check if monitored site is accessible and safe"""
        try:
            response = requests.get(site_url, timeout=10)
            is_healthy = response.status_code == 200
            
            logger.info(f"Site {site_id} health check: {'PASS' if is_healthy else 'FAIL'}")
            
            return {
                'site_id': site_id,
                'url': site_url,
                'status_code': response.status_code,
                'is_healthy': is_healthy,
                'timestamp': datetime.utcnow()
            }
        except Exception as e:
            logger.warning(f"Site {site_id} health check error: {e}")
            return {
                'site_id': site_id,
                'url': site_url,
                'status_code': 0,
                'is_healthy': False,
                'timestamp': datetime.utcnow()
            }

    def detect_key_compromise(self, wallet_address):
        """Detect if wallet key has been compromised based on suspicious patterns"""
        try:
            # Query blockchain for suspicious activity patterns
            # This is a placeholder - implement your detection logic
            logger.info(f"Checking key compromise for wallet: {wallet_address}")
            
            return {
                'wallet': wallet_address,
                'is_compromised': False,
                'risk_score': 0,
                'timestamp': datetime.utcnow()
            }
        except Exception as e:
            logger.error(f"Key compromise detection error: {e}")
            return None

    def log_monitoring_event(self, event_type, event_data):
        """Log monitoring event to database"""
        try:
            conn = self.get_db_connection()
            with conn.cursor() as cur:
                cur.execute(
                    """
                    INSERT INTO monitoring_events (event_type, event_data, timestamp)
                    VALUES (%s, %s, %s)
                    """,
                    (event_type, str(event_data), datetime.utcnow())
                )
            conn.commit()
            conn.close()
        except Exception as e:
            logger.error(f"Error logging event: {e}")

    async def run_monitoring_cycle(self):
        """Execute one complete monitoring cycle"""
        logger.info("Starting monitoring cycle...")
        
        sites = self.get_monitored_sites()
        logger.info(f"Monitoring {len(sites)} sites")
        
        for site in sites:
            # Check site health
            health = self.check_site_health(site['url'], site['id'])
            self.log_monitoring_event('site_health_check', health)
            
            # Check for key compromise
            if 'wallet_address' in site:
                compromise = self.detect_key_compromise(site['wallet_address'])
                if compromise:
                    self.log_monitoring_event('key_compromise_check', compromise)
                    
                    if compromise['is_compromised']:
                        logger.warning(f"KEY COMPROMISE DETECTED: {site['wallet_address']}")
                        self.log_monitoring_event('compromise_alert', compromise)
        
        logger.info("Monitoring cycle complete")

    async def start(self):
        """Start continuous monitoring"""
        logger.info(f"Titan Site Monitor started - checking every {self.monitor_interval}s")
        
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
    monitor = TitanSiteMonitor()
    asyncio.run(monitor.start())
