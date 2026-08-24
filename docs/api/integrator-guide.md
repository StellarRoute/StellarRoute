# Integrator Guide: Embedding Swaps

This guide provides third-party applications with integration steps and a pre-flight checklist for safely embedding StellarRoute swaps in production environments.

---

## Go-Live Checklist for Embedding Swaps

Complete the following verification steps before enabling production traffic:

### 1. Production Credentials & Network Setup
* **API Keys:** Replace testnet credentials with live production API keys.
* **Network Endpoints:** Direct Horizon and Soroban RPC requests to production endpoints.
* **CORS & Domain Whitelisting:** Ensure your application domain is allowlisted for API communication and webhook events.

### 2. Sample Production Environment (`.env.production`)

```env
STELLAR_NETWORK=PUBLIC
STELLAR_HORIZON_URL=[https://horizon.stellar.org](https://horizon.stellar.org)
STELLAR_SOROBAN_RPC_URL=[https://mainnet.soroban.rpc.org](https://mainnet.soroban.rpc.org)
STELLARROUTE_API_KEY=sr_live_your_production_api_key
STELLARROUTE_API_URL=[https://api.stellarroute.io/v1](https://api.stellarroute.io/v1)