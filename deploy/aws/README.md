# AWS deploy assets

Terraform + scripts for the StellarRoute backend on AWS (ECS Fargate).

**Operator runbook:** [`docs/deployment/aws.md`](../../docs/deployment/aws.md)

```bash
# 1. Configure
cd terraform
cp terraform.tfvars.example terraform.tfvars

# 2. Apply
terraform init && terraform apply

# 3. Secrets (router + admin + CORS)
../scripts/update-secrets.sh

# 4. Push images (from repo root)
../scripts/push-images.sh

# 5. Smoke
STAGING_API_BASE_URL=https://api.example.com ../../scripts/staging-smoke.sh
```
