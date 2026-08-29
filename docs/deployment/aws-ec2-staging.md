# AWS EC2 Staging Deployment

For one staging deployment, this is the best AWS path for StellarRoute.

Why this path wins:

- Lowest fixed monthly cost on AWS
- Simpler than ECS + ALB + RDS + ElastiCache
- Reuses the repo's existing Docker Compose production overlay
- Good enough for one low-traffic staging environment

This is not the most production-shaped AWS setup. It is the most efficient one
for a single staging environment.

Related files:

- `deploy/docker-compose.prod.yml`
- `deploy/env.prod.example`
- `deploy/aws/caddy/Caddyfile.example`
- `deploy/aws/scripts/bootstrap-ec2.sh`
- `deploy/aws/scripts/deploy-ec2-staging.sh`
- `deploy/aws/scripts/ec2-staging-start.sh`
- `deploy/aws/scripts/ec2-staging-stop.sh`
- `deploy/aws/scripts/ec2-postdeploy-smoke.sh`
- `deploy/aws/scripts/ec2-health-check.sh`
- `deploy/aws/scripts/install-caddy.sh`
- `.github/workflows/deploy-ec2-staging.yml`
- `deploy/aws/scripts/postgres-backup.sh`
- `deploy/aws/scripts/postgres-restore.sh`
- `deploy/aws/systemd/stellarroute-healthcheck.timer`
- `deploy/aws/systemd/stellarroute-postgres-backup.timer`
- `deploy/aws/systemd/stellarroute-staging.service`
- `docs/deployment/vercel-frontend.md`
- `docs/deployment/aws-staging-cost-estimate.md`

## Recommended instance

Start with one of these Graviton instances in `us-east-1`:

- `t4g.small` for the cheapest start
- `t4g.medium` if you want more headroom for builds, indexer bursts, and Docker memory pressure

Suggested storage:

- 30-50 GB gp3 EBS

Suggested OS:

- Ubuntu 24.04 LTS

## Security group

Allow inbound:

- `22/tcp` from your IP only
- `80/tcp` from `0.0.0.0/0` only if using HTTP-to-HTTPS redirect on-box
- `443/tcp` from `0.0.0.0/0` if terminating TLS on the instance

Do not allow inbound:

- `5432/tcp`
- `6379/tcp`

Outbound can remain open.

## Cost shape

The EC2 single-host path is cheaper than the ECS path because it avoids:

- ALB fixed monthly cost
- NAT Gateway fixed monthly cost
- RDS fixed monthly cost
- ElastiCache fixed monthly cost

Directional monthly range for one always-on staging VM:

- `t4g.small` + 30-50 GB gp3: roughly `$18-$35/month`
- `t4g.medium` + 30-50 GB gp3: roughly `$28-$50/month`

These estimates exclude unusual bandwidth spikes and optional extras such as
Route 53, CloudWatch retention beyond basics, or external monitoring vendors.

## 1. Create the instance

In EC2:

1. Launch one Ubuntu 24.04 LTS ARM instance.
2. Instance type: `t4g.small` or `t4g.medium`.
3. Root volume: gp3, 30-50 GB.
4. Attach the security group described above.
5. Attach your SSH key.
6. Create a DNS record such as `api-staging.<your-domain>` pointing at the instance public IP before enabling Caddy.

## 2. Bootstrap the host

SSH into the instance and run:

```bash
sudo bash deploy/aws/scripts/bootstrap-ec2.sh
```

If the repo is not on the box yet:

```bash
sudo mkdir -p /opt
sudo chown "$USER":"$USER" /opt
git clone https://github.com/StellarRoute/StellarRoute.git /opt/stellarroute
cd /opt/stellarroute
sudo bash deploy/aws/scripts/bootstrap-ec2.sh
```

Log out and back in so the `docker` group applies.

## 3. Fill environment variables

From the repo root on the instance:

```bash
cd /opt/stellarroute
cp deploy/env.prod.example .env.prod
```

Fill at least:

- `POSTGRES_USER`
- `POSTGRES_PASSWORD`
- `POSTGRES_DB`
- `REDIS_PASSWORD`
- `SOROBAN_RPC_URL`
- `STELLAR_HORIZON_URL`
- `ROUTER_CONTRACT_ADDRESS`
- `CORS_ALLOWED_ORIGINS`
- `ADMIN_AUTH_TOKEN`

Keep:

- `STELLARROUTE_ENV=production`
- `ENABLE_ADMIN_ROUTES=false`
- `API_HOST_PORT=8080`

## 4. Deploy the stack

Run:

```bash
cd /opt/stellarroute
bash deploy/aws/scripts/deploy-ec2-staging.sh
```

This validates `.env.prod`, renders the merged Compose config, builds the
images, and starts the API, indexer, Postgres, and Redis.

## 5. Install the systemd unit

This makes the stack come back after a reboot.

```bash
cd /opt/stellarroute
sudo cp deploy/aws/systemd/stellarroute-staging.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now stellarroute-staging.service
sudo systemctl status stellarroute-staging.service
```

The unit assumes the repo lives at `/opt/stellarroute`.

## 5b. Enable backups and basic monitoring

For the single-host setup, use local compressed Postgres dumps plus lightweight
systemd timers.

Install the timers:

```bash
cd /opt/stellarroute
sudo cp deploy/aws/systemd/stellarroute-postgres-backup.service /etc/systemd/system/
sudo cp deploy/aws/systemd/stellarroute-postgres-backup.timer /etc/systemd/system/
sudo cp deploy/aws/systemd/stellarroute-healthcheck.service /etc/systemd/system/
sudo cp deploy/aws/systemd/stellarroute-healthcheck.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now stellarroute-postgres-backup.timer
sudo systemctl enable --now stellarroute-healthcheck.timer
systemctl list-timers --all | grep stellarroute
```

What they do:

- `stellarroute-postgres-backup.timer` runs daily and writes compressed dumps to `/var/backups/stellarroute-postgres`
- `stellarroute-healthcheck.timer` runs every 5 minutes and checks Docker, Caddy, disk pressure, and local API health

Run them manually if needed:

```bash
sudo bash deploy/aws/scripts/postgres-backup.sh
bash deploy/aws/scripts/ec2-health-check.sh
```

## 6. Expose the API with Caddy

This is the default recommendation for EC2 staging. It is simpler than managing
Nginx yourself and gives you automatic HTTPS.

## Optional: file-based indexer health sidecar

If you want a process-supervisor health signal without exposing an HTTP port, set
`INDEXER_HEALTH_FILE` on the indexer service. When the variable is unset, the
indexer keeps its exact default behavior and writes nothing.

Example:

```bash
export INDEXER_HEALTH_FILE=/var/run/stellarroute/indexer-health.json
```

When enabled, the indexer writes periodic JSON like:

```json
{"ok":true,"sdex_lag":3,"amm_lag":4,"ts":"2026-08-29T00:00:00Z"}
```

This is additive-only and does not change ingest semantics, quote ranking, swap
flow, or OpenAPI contracts. It is intended for EC2/Oracle supervisors that only
need a file-based liveness signal.

Requirements:

- your DNS record already points to the EC2 public IP
- ports `80` and `443` are open in the instance security group

Run on the instance:

```bash
cd /opt/stellarroute
sudo bash deploy/aws/scripts/install-caddy.sh api-staging.<your-domain> ops@<your-domain>
```

This installs the official Caddy Ubuntu package, writes `/etc/caddy/Caddyfile`,
formats it, and starts the `caddy` systemd service.

The generated proxy is equivalent to:

```caddyfile
api-staging.<your-domain> {
  encode zstd gzip
  reverse_proxy 127.0.0.1:8080
}
```

Template reference:

- `deploy/aws/caddy/Caddyfile.example`

### Alternate public path: Cloudflare Tunnel

If you do not want the instance directly reachable on `80/443`, use the tunnel
approach from:

- `docs/deployment/oracle-always-free.md`

Point the tunnel to `http://127.0.0.1:8080`.

## 7. Wire the frontend

In Vercel production:

- `NEXT_PUBLIC_API_URL=https://api.<your-domain>`
- `NEXT_PUBLIC_API_URL_TESTNET=https://api.<your-domain>`
- `NEXT_PUBLIC_STELLAR_NETWORK=testnet`

Then confirm `CORS_ALLOWED_ORIGINS` in `.env.prod` includes:

- `https://www.stellarroute.app`
- `https://stellarroute.app`
- your Vercel project hostname

See:

- `docs/deployment/vercel-frontend.md`

## 8. Verify

On the instance:

```bash
curl -sf http://127.0.0.1:8080/health && echo OK
curl -sf http://127.0.0.1:8080/health/deps && echo OK
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml --env-file .env.prod ps
```

From your laptop after the public hostname is live:

```bash
curl -sf https://api-staging.<your-domain>/health && echo OK
curl -sf https://api-staging.<your-domain>/health/deps && echo OK
STAGING_API_BASE_URL=https://api-staging.<your-domain> ./scripts/staging-smoke.sh
```

Or from the EC2 host, run the single-command smoke wrapper:

```bash
cd /opt/stellarroute
bash deploy/aws/scripts/ec2-postdeploy-smoke.sh https://api-staging.<your-domain>
```

## 9. Restore from a local Postgres backup

Backups are written by default to:

- `/var/backups/stellarroute-postgres`

Create one on demand:

```bash
cd /opt/stellarroute
sudo bash deploy/aws/scripts/postgres-backup.sh
```

Restore one backup:

```bash
cd /opt/stellarroute
sudo bash deploy/aws/scripts/postgres-restore.sh /var/backups/stellarroute-postgres/<backup-file>.sql.gz --yes
```

Restore behavior:

- stops API and indexer first to avoid write traffic
- drops and recreates the application database
- imports the selected dump
- restarts API and indexer

This is intentionally destructive to the current database contents, so use it
only for staging recovery or refresh workflows.

## 10. Start and stop staging to reduce cost

If you do not need 24/7 uptime, stop the EC2 instance when idle.

From your local machine:

```bash
cd /path/to/StellarRoute
bash deploy/aws/scripts/ec2-staging-start.sh
```

```bash
cd /path/to/StellarRoute
bash deploy/aws/scripts/ec2-staging-stop.sh
```

Defaults in these scripts:

- `AWS_REGION=us-east-1`
- `STAGING_INSTANCE_NAME=stellarroute-staging-ec2`

Override if needed:

```bash
AWS_REGION=us-east-1 STAGING_INSTANCE_ID=i-xxxxxxxxxxxxxxxxx bash deploy/aws/scripts/ec2-staging-start.sh
AWS_REGION=us-east-1 STAGING_INSTANCE_ID=i-xxxxxxxxxxxxxxxxx bash deploy/aws/scripts/ec2-staging-stop.sh
```

## Best practices for this shape

- Keep Postgres and Redis unexposed to the public internet.
- Keep SSH limited to your IP.
- Keep the API bound to `127.0.0.1:8080` behind Caddy or Cloudflare Tunnel.
- Snapshot the EBS volume or back up Postgres daily.
- Check `journalctl -u stellarroute-healthcheck.service -n 100` if the host starts failing health checks.
- Keep `ENABLE_ADMIN_ROUTES=false`.
- Do not use `latest` forever if you move beyond staging; pin deploys to a git SHA.
- If the instance is too tight during builds, move from `t4g.small` to `t4g.medium` before over-engineering the platform.

## When to leave EC2 and move to ECS

Move to ECS/RDS later if any of these become true:

- staging must closely mirror production
- you want managed Postgres durability and easier restores
- you need cleaner service isolation
- one VM becomes an operational bottleneck

## 11. Auto-update EC2 from GitHub (main branch)

This repo now includes:

- `.github/workflows/deploy-ec2-staging.yml`

What it does:

- triggers on pushes to `main` (for backend/deploy path changes)
- can also run manually from Actions via `workflow_dispatch`
- uses AWS Systems Manager (SSM) Run Command to execute deploy steps on EC2
- runs:
  - `git fetch` + `git checkout` + `git pull --ff-only`
  - `bash deploy/aws/scripts/deploy-ec2-staging.sh`
  - `bash deploy/aws/scripts/ec2-postdeploy-smoke.sh <staging-url>`

Why SSM instead of SSH from Actions:

- no inbound port `22` required for GitHub runners
- avoids dynamic GitHub runner IP allowlist maintenance

Required GitHub repository secrets:

- `AWS_ROLE_ARN` (IAM role assumed by OIDC)
- `STAGING_INSTANCE_ID` (for example, `i-073051e6b1dccd329`)
- optional `AWS_REGION` (defaults to `us-east-1`)
- one of:
  - `STAGING_API_BASE_URL` (recommended, for example `https://34.224.110.144.sslip.io`)
  - or `STAGING_EC2_HOST` (workflow will derive `https://<host>.sslip.io`)

IAM role policy must allow at least:

- `ssm:SendCommand`
- `ssm:GetCommandInvocation`
- `ssm:ListCommandInvocations`
- `ec2:DescribeInstances`

The EC2 instance must be managed by SSM:

- SSM Agent installed/running (Ubuntu AMIs usually include this)
- instance IAM role/profile allowing SSM agent registration (for example `AmazonSSMManagedInstanceCore`)

Manual run:

1. Open GitHub Actions.
2. Run workflow `Deploy EC2 Staging`.
3. (Optional) set `git_ref`.
