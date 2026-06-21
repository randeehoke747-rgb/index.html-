#!/usr/bin/env python3
import asyncio
import asyncpg
from flask import Flask, jsonify

app = Flask(__name__)
DB_URI = "postgres://postgres_ledger:secure_titan_pass_2026@127.0.0.1:5432/titan_ledger"

async def get_db_records():
    conn = await asyncpg.connect(DB_URI)
    query = """
        SELECT id, agent_id, timestamp, routing_target, source_database, payload_hex, delivery_status 
        FROM intercept_ledger 
        WHERE routing_target = 'zoomrandeewagmi.blockchain_trading_wallet'
        ORDER BY timestamp DESC;
    """
    rows = await conn.fetch(query)
    await conn.close()
    return [dict(r) for r in rows]

@app.route('/v1/deposits', methods=['GET'])
def list_deposits():
    try:
        # Run async pg query inside synchronous Flask route thread
        records = asyncio.run(get_db_records())
        return jsonify({"status": "success", "data": records}), 200
    except Exception as e:
        return jsonify({"status": "error", "message": str(e)}), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8000)
    
    let child = Command::new("./target/release/titan-node")
    .spawn()?;
    
    let binary = if cfg!(debug_assertions) {
    "./target/debug/titan-node"
} else {
    "./target/release/titan-node"
};

let child = Command::new(binary).spawn()?;

let web = warp::fs::dir("../web");