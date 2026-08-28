# AWS Backend Deployment (API + Indexer)

Deploy StellarRoute’s **backend** (API, indexer, Postgres, Redis) on AWS using
**ECS Fargate**, **RDS PostgreSQL**, **ElastiCache Redis**, **Application Load
Balancer**, **ECR**, and **Secrets Manager**.

Frontend stays on **Vercel** ([vercel-frontend.md](./vercel-frontend.md)).
This runbook is the AWS counterpart to [oracle-always-free.md](./oracle-always-free.md)
and the Render blueprint (`render.yaml`).

| Item | Path |
|------|------|
| Terraform | `deploy/aws/terraform/` |
| Image push script | `deploy/aws/scripts/push-images.sh` |
| Secrets helper | `deploy/aws/scripts/update-secrets.sh` |
| Env / secret template | `deploy/env.aws.example` |
| Operator checklist | `deploy/secrets.checklist.md` (§ AWS) |
| Smoke test | `scripts/staging-smoke.sh` |

## Assumptions (defaults)

| Choice | Default | Notes |
|--------|---------|--------|
| Compute | **ECS Fargate** | Managed containers; no SSH boxes |
| IaC | **Terraform** | `deploy/aws/terraform` |
| First env | **Testnet staging** | Horizon/Soroban testnet + Vercel |
| Region | `us-east-1` | Override with `var.aws_region` |
| Images | **ECR** | Built from `Dockerfile.api` / `Dockerfile.indexer` |

Alternative (not primary): EC2 + `deploy/docker-compose.prod.yml` — same as Oracle,
with AWS Secrets Manager / SSM for env. Use only if you need a single VM.

## Architecture

```
Internet
   │
   ▼
Application Load Balancer (HTTPS :443)
   │
   ▼
ECS Fargate — stellarroute-api   (PORT=8080, /health/deps)
   │                │
   │                ├─► RDS PostgreSQL (private subnet)
   │                └─► ElastiCache Redis (private subnet)
   │
ECS Fargate — stellarroute-indexer (no public port)
   │
   └─► Horizon + Soroban RPC + ROUTER_CONTRACT_ADDRESS
```

- API is the only public service (via ALB).
- Postgres and Redis have **no** public endpoints.
- Indexer is a private Fargate service (worker).
- Secrets live in **AWS Secrets Manager**; task roles pull them at start.

## Cost sketch (staging)

Rough monthly for a small staging footprint (us-east-1, always-on):

| Resource | Example size | Order of magnitude |
|----------|--------------|--------------------|
| ECS Fargate API | 0.5 vCPU / 1 GB | Low–mid tens of USD |
| ECS Fargate indexer | 0.5 vCPU / 1 GB | Low–mid tens of USD |
| RDS Postgres | `db.t4g.micro`, 20 GB gp3 | Mid tens of USD |
| ElastiCache Redis | `cache.t4g.micro` | Mid tens of USD |
| ALB + NAT + data | — | Often the largest variable |

Use NAT Gateway carefully (one NAT is simpler, two AZs cost more). For
cheapest experiments, Oracle Always Free remains valid; AWS is the path for
production-shaped reliability and IAM/secrets.

## Prerequisites

1. AWS account + IAM principal that can create VPC/ECS/RDS/ECR/ALB/Secrets.
2. AWS CLI v2 configured (`aws sts get-caller-identity`).
3. Terraform `>= 1.5`.
4. Docker (to build/push images).
5. Domain (optional but recommended) in Route 53 or external DNS for the ALB.
6. ACM certificate in the **same region** as the ALB (for HTTPS).
7. Testnet router ID in `config/deployments/testnet.json`.

## Quick start

### 1. Configure Terraform

```bash
cd deploy/aws/terraform
cp terraform.tfvars.example terraform.tfvars
# Edit: project_name, aws_region, domain/certificate_arn, alert email (optional)
```

### 2. Initialize and apply network + data plane first

```bash
terraform init
terraform plan -out=tfplan
terraform apply tfplan
```

Outputs include:

- `ecr_api_repository_url` / `ecr_indexer_repository_url`
- `alb_dns_name`
- `secrets_arn`
- `ecs_cluster_name`

### 3. Fill application secrets

Terraform seeds Secrets Manager with RDS/Redis URLs and placeholder auth/router
values. Replace them before public traffic:

```bash
# From repo root — reads terraform outputs + config/deployments/testnet.json
./deploy/aws/scripts/update-secrets.sh
```

Or hand-edit via `aws secretsmanager put-secret-value` using keys in
`deploy/env.aws.example`.

### 4. Build and push images to ECR

```bash
# From repo root
./deploy/aws/scripts/push-images.sh
```

Tag defaults to git SHA + `latest`. Override with `IMAGE_TAG=...`.

### 5. Force a new ECS deployment

```bash
cd deploy/aws/terraform
aws ecs update-service \
  --cluster "$(terraform output -raw ecs_cluster_name)" \
  --service "$(terraform output -raw api_service_name)" \
  --force-new-deployment
aws ecs update-service \
  --cluster "$(terraform output -raw ecs_cluster_name)" \
  --service "$(terraform output -raw indexer_service_name)" \
  --force-new-deployment
```

Or re-apply Terraform after changing `image_tag` in `terraform.tfvars`.

### 6. Point DNS + Vercel

1. Create a CNAME (or alias) from `api.<your-domain>` → `alb_dns_name`.
2. Confirm ACM cert covers that hostname.
3. In Vercel production env:
   - `NEXT_PUBLIC_API_URL=https://api.<your-domain>`
   - `NEXT_PUBLIC_API_URL_TESTNET=https://api.<your-domain>`
4. Ensure `CORS_ALLOWED_ORIGINS` includes the live frontend origins.

### 7. Verify

```bash
curl -sf https://api.<your-domain>/health && echo OK
curl -sf https://api.<your-domain>/health/deps && echo OK
STAGING_API_BASE_URL=https://api.<your-domain> ./scripts/staging-smoke.sh
```

ALB target health uses **`GET /health/deps`** (DB/Redis). `GET /health` can
return **503** while indexer lag is critical — do not use it alone as the sole
deploy gate for brand-new empty DBs.

## Environment variables

| Key | Service | Source |
|-----|---------|--------|
| `DATABASE_URL` | API, Indexer | Secrets Manager (Terraform → RDS) |
| `REDIS_URL` | API | Secrets Manager (Terraform → ElastiCache `redis://`, VPC-private; no TLS yet) |
| `PORT` | API | Task def `8080` |
| `STELLARROUTE_ENV` | API | `production` |
| `CORS_ALLOWED_ORIGINS` | API | Secrets Manager |
| `PUBLIC_GET_ROUTES` | API | Secrets Manager |
| `ADMIN_AUTH_TOKEN` | API | Secrets Manager |
| `ENABLE_ADMIN_ROUTES` | API | Task def `false` |
| `SOROBAN_RPC_URL` | API, Indexer | Secrets Manager |
| `STELLAR_HORIZON_URL` | Indexer (+ API) | Secrets Manager |
| `ROUTER_CONTRACT_ADDRESS` | Indexer | Secrets Manager |
| `AMM_POOLS` | Indexer | Secrets Manager (optional) |
| `RUST_LOG` | Both | Task def |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Both | Optional secret / env |

Production posture matches [README.md](./README.md#api-production-security-m5):
non-empty CORS allowlist, admin token, admin routes off by default.

## CI / images

- GitHub already builds to **GHCR** via `.github/workflows/docker-images.yml`.
- For AWS, prefer **ECR** via `deploy/aws/scripts/push-images.sh` or the
  optional workflow `.github/workflows/deploy-aws-ecr.yml` (requires AWS OIDC
  roles — see that file’s comments).

Suggested promote flow:

1. Merge to `main` → GHCR images for audit.
2. `push-images.sh` (or ECR workflow) → ECR.
3. ECS force-new-deployment / Terraform `image_tag` bump.

## Operations

### Logs

```bash
aws logs tail /ecs/<project>-api --follow
aws logs tail /ecs/<project>-indexer --follow
```

Log group names are Terraform outputs / `aws_cloudwatch_log_group` resources.

### Scaling

- API: raise `api_desired_count` / CPU/memory in `terraform.tfvars`.
- Indexer: usually **desired_count = 1** (single writer). Do not scale indexer
  horizontally without a leader-election design.

### Migrations

Indexer/API apply SQL migrations on startup (existing behavior). Follow
[migration-runbook.md](./migration-runbook.md) for expand/contract changes.
Prefer running a one-off ECS task or local migrate against RDS in a VPN/bastion
before flipping risky schema.

### Indexer lag

If `/health` is 503 for SDEX/AMM lag, check indexer logs and Horizon/RPC.
The API lag monitor prefers `ingestion_state.sdex_last_horizon_ledger` over
stale offer ledgers — keep the indexer running.

### Rollback

1. Redeploy previous ECR image tag (`image_tag` in Terraform or
   `update-service` with older task definition).
2. Keep prior RDS snapshot before major upgrades (`backup_retention_period`).

### Destroy (staging only)

```bash
cd deploy/aws/terraform
terraform destroy
```

RDS deletion protection defaults to **on** for non-dev; set
`rds_deletion_protection = false` only for disposable staging.

## Security checklist

- [ ] No public SG rules for Postgres (5432) or Redis (6379) from `0.0.0.0/0`
- [ ] Redis is private-subnet only (in-transit TLS deferred until the Redis client enables TLS features)
- [ ] ALB HTTPS only (redirect HTTP → HTTPS)
- [ ] Secrets only in Secrets Manager (never in task def plaintext or git)
- [ ] `ENABLE_ADMIN_ROUTES=false` until kill-switch review is done
- [ ] Separate AWS accounts or at least separate Terraform workspaces for
      staging vs mainnet production
- [ ] IAM: least-privilege deploy role; no long-lived access keys in GitHub if
      OIDC is available

## Troubleshooting

| Symptom | Likely cause | Action |
|---------|--------------|--------|
| ALB targets unhealthy | `/health/deps` failing | Check `DATABASE_URL` / `REDIS_URL`, SG rules, task logs |
| API 503 on `/health` | Indexer lag critical | Check indexer service; Horizon/RPC; `ingestion_state` |
| Indexer crash loop | Missing `ROUTER_CONTRACT_ADDRESS` | Fix secret; confirm testnet.json |
| CORS errors in browser | Origin not allowlisted | Update `CORS_ALLOWED_ORIGINS` secret + redeploy API |
| Quote “Market data still updating” | Empty/stale liquidity | Confirm indexer sync; staging pairs (e.g. BTC/EXT) |
| Cannot pull image | Wrong ECR region/account or missing `ecsTaskExecutionRole` pull rights | Check execution role + repo policy |

## Related docs

- [secrets-management.md](./secrets-management.md)
- [oracle-always-free.md](./oracle-always-free.md) (free Wave 0 path)
- [gradual-rollout-plan.md](./gradual-rollout-plan.md)
- [vercel-frontend.md](./vercel-frontend.md)
- `deploy/docker-compose.prod.yml` (VM / Compose equivalent)
