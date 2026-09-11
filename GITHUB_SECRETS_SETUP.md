# GitHub Secrets Setup for Titan Security Monitoring

## How to Add Secrets to GitHub Actions

1. Go to your repository: `https://github.com/randeehoke747-rgb/index.html-`
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add each secret below:

### Required Secrets

| Secret Name | Value | Description |
|-------------|-------|-------------|
| `ETHEREUM_RPC_URL` | Your Alchemy/Infura API endpoint | Ethereum node RPC connection |
| `ETHEREUM_CHAIN_ID` | `1` | Mainnet chain ID |
| `SAFE_WALLET_ADDRESS` | `0xCE4A926070c0544052de1E56d1254eaac79b77be` | USDC recovery wallet |
| `USDT_WALLET_ADDRESS` | `0x5464A4b1f40381D3de7e70520403067C8452ac44` | Retained USDT wallet |
| `USDC_CONTRACT_ADDRESS` | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | USDC token contract |
| `AGENT_PRIVATE_KEY_ENCRYPTED` | Your encrypted private key | For fund transfers (use encryption) |
| `DATABASE_URL` | PostgreSQL connection string | Transaction & monitoring database |
| `CLIENT_AUTH_SECRET` | Your client auth secret | For verifying client authorization |

### Optional Secrets

| Secret Name | Value | Description |
|-------------|-------|-------------|
| `ALCHEMY_API_KEY` | Your Alchemy API key | For Ethereum monitoring |
| `SLACK_WEBHOOK_URL` | Your Slack webhook | For alerts |
| `LOG_LEVEL` | `INFO` or `DEBUG` | Agent logging level |

## Security Best Practices

✅ **DO:**
- Use encrypted private keys
- Rotate secrets regularly
- Use separate keys per environment (dev/prod)
- Enable branch protection before deploying
- Require signatures for fund transfers

❌ **DON'T:**
- Commit `.env` or secrets to repository
- Use hardcoded private keys
- Share secrets via Slack/Email
- Store unencrypted keys anywhere

## Testing Secrets in Workflow

The workflow will use these secrets automatically. To verify they're loaded:

```bash
echo "Secrets loaded: ${{ secrets.SAFE_WALLET_ADDRESS }}"
echo "USDT wallet loaded: ${{ secrets.USDT_WALLET_ADDRESS }}"
```
