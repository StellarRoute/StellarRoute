# AWS Staging Cost Estimate

This estimate is for one always-on staging deployment of StellarRoute on AWS.

For this repo, if you only need one staging environment, a single EC2 instance
is usually the best AWS choice. The ECS numbers below are still useful if you
want a more production-shaped staging stack.

Scope:

- Frontend remains on Vercel.
- Backend runs in AWS.
- One API task.
- One indexer task.
- One PostgreSQL instance.
- No Redis for the cheapest staging profile.
- No NAT Gateway for the cheapest staging profile.

These numbers are directional, not a bill guarantee. They vary by region, data
transfer, storage growth, log volume, and request rate. The main purpose is to
show the cost shape and the biggest levers.

## Pricing assumptions checked against AWS docs

- ECS on Fargate supports mixing Fargate and Fargate Spot with capacity
  providers, and Spot tasks can be interrupted with a two-minute notice.
- RDS PostgreSQL supports the `db.t4g` burstable Graviton family, which is the
  right low-cost default for a small staging database.
- NAT Gateway is a fixed hourly charge plus per-GB processing, so it is often a
  disproportionate cost for small always-on environments.
- ElastiCache is billed per node, so even the smallest dedicated Redis node is
  a meaningful fixed monthly cost.

## Recommended staging profile

Use this profile for the first AWS staging deployment:

- ECS API: Fargate, `256` CPU, `512` MiB, `desired_count = 1`
- ECS indexer: Fargate Spot, `256` CPU, `512` MiB, `desired_count = 1`
- RDS PostgreSQL: `db.t4g.micro`, 20 GB gp3
- Redis: disabled
- NAT Gateway: disabled
- ECS tasks: public subnets with strict security groups
- ALB: enabled for the API

## Monthly estimate

### Best single-staging option on AWS: one EC2 instance

| Component | Configuration | Estimated monthly |
| --- | --- | ---: |
| EC2 compute | `t4g.small` to `t4g.medium` | $12-$30 |
| EBS storage | 30-50 GB gp3 | $3-$8 |
| Data transfer and logs | modest staging traffic | $2-$10 |
| Total | single-host staging | **$17-$48** |

This path uses the VM deployment flow in `docs/deployment/aws-ec2-staging.md`
and avoids the fixed monthly cost of ALB, NAT Gateway, RDS, and ElastiCache.

### Cheapest sensible staging on AWS

| Component | Configuration | Estimated monthly |
| --- | --- | ---: |
| ALB | 1 public ALB for API | $18-$30 |
| ECS API | Fargate 0.25 vCPU / 0.5 GB | $9-$14 |
| ECS indexer | Fargate Spot 0.25 vCPU / 0.5 GB | $3-$6 |
| RDS PostgreSQL | `db.t4g.micro` + 20 GB gp3 | $16-$28 |
| CloudWatch logs | low traffic / basic retention | $2-$8 |
| Data transfer | modest staging traffic | $2-$10 |
| Total | recommended starting point | **$50-$96** |

### Safer staging with private ECS networking

This is the same stack, but ECS tasks stay in private subnets and use one NAT
Gateway for outbound access.

| Component | Configuration | Estimated monthly |
| --- | --- | ---: |
| Cheapest sensible staging total | from above | $50-$96 |
| NAT Gateway | 1 gateway + light processing | $32-$45 |
| Total | private ECS subnets | **$82-$141** |

### Managed-cache staging

This is the safer staging profile plus a small ElastiCache Redis node.

| Component | Configuration | Estimated monthly |
| --- | --- | ---: |
| Safer staging total | from above | $82-$141 |
| ElastiCache Redis | `cache.t4g.micro` | $12-$20 |
| Total | with managed Redis | **$94-$161** |

## Main cost drivers

For this repo, the largest cost levers are usually:

1. NAT Gateway
2. ALB fixed cost
3. RDS fixed cost
4. Whether Redis is enabled

Fargate CPU and memory matter, but for a small staging environment they are not
usually the biggest surprise line item.

## Recommendation

For one AWS staging deployment, start with the cheapest sensible profile:

- No NAT Gateway
- No Redis
- API on standard Fargate
- Indexer on Fargate Spot
- `db.t4g.micro` PostgreSQL

Then add NAT and Redis only when traffic, reliability, or security posture
needs justify the extra fixed monthly cost.

## Repo files tied to this estimate

- `deploy/aws/terraform/terraform.staging.lowcost.tfvars`
- `deploy/aws/terraform/variables.tf`
- `deploy/aws/terraform/ecs.tf`
- `deploy/aws/terraform/data.tf`
- `docs/deployment/aws.md`
