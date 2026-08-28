```markdown
# Incident Response Runbook: Live Trading Outage

This runbook defines operational response procedures, severity tiers, diagnostic commands, mitigation workflows, and status communication guidelines for live trading incidents affecting StellarRoute.

> **CRITICAL INVARIANT:** Under no circumstances should on-call operators modify quote ranking formulas, routing math, slippage logic, or transaction-building handlers during an active incident. Mitigate exclusively via kill-switches, upstream RPC failovers, indexer restarts, or infrastructure scaling.

---

## 1. Incident Severity Matrix

| Severity | Definition & Indicators | Response SLA | Target Mitigation | Communication Cadence |
| :--- | :--- | :--- | :--- | :--- |
| **SEV-1 (Critical)** | Complete trading halt, Horizon RPC 5xx / unreachable, API `/health` returning non-200, swap preparation failing across all pairs. | < 5 mins | < 15 mins | Every 15 mins |
| **SEV-2 (Major)** | Degraded routing performance, Indexer lag > 100 ledgers, Redis cache outage (direct database/RPC fallback active), latency P95 > 2s, specific AMM pool/venue failing. | < 15 mins | < 45 mins | Every 30 mins |
| **SEV-3 (Minor)** | Isolated RPC provider rate-limiting with healthy fallback, non-blocking telemetry delays, minor UI status badge sync lag. | < 1 hour | < 4 hours | Upon resolution |

---

## 2. Roles and Escalation

- **Incident Commander (IC):** Lead On-Call Engineer. Holds final authority to declare severity, execute kill-switches, and approve upstream failovers.
- **Operations / Infrastructure Lead:** Executes deployment restarts, monitors Redis/DB connection pools, and tracks ledger ingestion sync.
- **Communications Lead:** Updates the `/status` page, notifies ecosystem integrators, and coordinates public incident updates.

---

## 3. Triage & Health Diagnostics

Execute the standard diagnostic sequence:

```bash
# 1. Basic API health check
curl -s -i [https://api.stellarroute.io/health](https://api.stellarroute.io/health)

# 2. Detailed subsystem connectivity and latency
curl -s [https://api.stellarroute.io/health/detailed](https://api.stellarroute.io/health/detailed) | jq .

# 3. Indexer sync and ledger height check
curl -s [https://api.stellarroute.io/v1/indexer/status](https://api.stellarroute.io/v1/indexer/status) | jq .

# 4. View active kill-switch status
curl -s [https://api.stellarroute.io/api/v1/admin/kill-switch](https://api.stellarroute.io/api/v1/admin/kill-switch) | jq .

```

---

## 4. Scenario Mitigation Playbooks

### Scenario A: API Service Down or Unresponsive (SEV-1)

1. **Engage Kill-Switch / Pause Ingress:**
* If incoming traffic is causing cascading crash loops, disable unstable sources/venues via [`docs/RUNBOOK_KILL_SWITCH.md`](https://www.google.com/search?q=../RUNBOOK_KILL_SWITCH.md).


2. **Post Status Update:** Post `Investigating: Core routing API degraded` on `/status`.
3. **Inspect Pod Logs:**
```bash
kubectl logs -n production -l app=stellarroute-api --tail=200

```


4. **Perform Rollout Restart:**
```bash
kubectl rollout restart deployment/stellarroute-api -n production

```


5. **Verify Recovery:** Ensure `HTTP 200` on `/health` before releasing any active kill-switches.

---

### Scenario B: Horizon RPC 5xx / Ingestion Failure (SEV-1)

1. **Verify Upstream Status:**
```bash
curl -s -i [https://horizon.stellar.org/](https://horizon.stellar.org/)

```

2. **Switch to Standby Horizon Provider:**
* Update the `HORIZON_URL` environment configuration to point to the secondary Horizon cluster or private validator node pool.


3. **Verify Ledger Alignment:**
* Ensure the fallback RPC node is caught up with the latest public ledger sequence.


4. **Flush Stale Connection Pools:**
* Restart API pods to clear stagnant HTTP connection keep-alives.



---

### Scenario C: Indexer Lag / Stale Liquidity Reserves (SEV-2)

1. **Assess Ledger Divergence:**
* If `current_network_ledger - last_indexed_ledger > 50`, routing engine calculations risk using stale AMM pool reserves.


2. **Engage Source Kill-Switch (Temporary):**
* If AMM reserve data is stale, temporarily disable the `amm` source via kill-switch while preserving SDEX orderbook routing:
```bash
curl -X POST [https://api.stellarroute.io/api/v1/admin/kill-switch](https://api.stellarroute.io/api/v1/admin/kill-switch) \
  -H "Content-Type: application/json" \
  -d '{"sources": {"amm": "force_exclude"}, "venues": {}}'

```


3. **Restart Indexer Ingestion:**
```bash
kubectl rollout restart deployment/stellarroute-indexer -n production

```


4. **Monitor Catchup:** Track `stellarroute_indexer_ledger_lag` until lag is < 5 ledgers, then clear the kill-switch.

---

### Scenario D: Redis Cache Outage / High Latency (SEV-2)

1. **Verify Direct Fallback:**
* The API will log cache misses and automatically query PostgreSQL and Horizon directly.


2. **Inspect Memory and Client Connections:**
```bash
redis-cli -u "$REDIS_URL" info memory
redis-cli -u "$REDIS_URL" info clients

```


3. **Trigger Cache Restart / Sentinel Failover:**
```bash
kubectl rollout restart statefulset/redis -n production

```


4. **Monitor Database Connection Pool:** Ensure DB connections remain below maximum capacity during cache warm-up.

---

## 5. Kill-Switch Operations

For complete instructions on inspecting, enabling, and clearing source or venue exclusions:

* [`docs/RUNBOOK_KILL_SWITCH.md`](https://www.google.com/search?q=../RUNBOOK_KILL_SWITCH.md)

**Authorized Operators:** Incident Commander, Lead On-Call Engineer.

---

## 6. Status Page & Communications Templates

* **Investigating (within 5 mins of SEV-1/SEV-2):**
> *"We are investigating reports of intermittent swap quote and routing availability. In-flight transactions are being monitored and safety controls are active."*


* **Identified (within 15 mins):**
> *"The issue has been identified as upstream Horizon RPC connectivity degradation. Traffic is being routed to backup infrastructure."*


* **Monitoring (post-mitigation):**
> *"Operational failover completed. Swap routing and execution are fully operational. We are monitoring quote generation latencies."*


* **Resolved:**
> *"All services are operating nominally. An incident post-mortem will be published within 48 hours."*



---

## 7. Related Documentation

* **Kill-Switch Runbook:** [`docs/RUNBOOK_KILL_SWITCH.md`](https://www.google.com/search?q=../RUNBOOK_KILL_SWITCH.md)
* **Monitoring & Alerting:** [`docs/monitoring.md`](https://www.google.com/search?q=../monitoring.md)
* **Indexer Lag Monitoring:** [`docs/indexer-lag-monitoring.md`](https://www.google.com/search?q=../indexer-lag-monitoring.md)

