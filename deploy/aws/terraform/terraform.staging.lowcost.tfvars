aws_region   = "us-east-1"
project_name = "stellarroute"
environment  = "staging"

# Must match region AZs
availability_zones = ["us-east-1a", "us-east-1b"]

# Fill this once ACM is issued in us-east-1 for api.<your-domain>
certificate_arn = ""

# Pin these to a git SHA after the first image push.
api_image_tag     = "latest"
indexer_image_tag = "latest"

# Lowest-cost always-on staging profile.
api_cpu           = 256
api_memory        = 512
api_desired_count = 1

indexer_cpu           = 256
indexer_memory        = 512
indexer_desired_count = 1

db_instance_class       = "db.t4g.micro"
db_allocated_storage    = 20
rds_deletion_protection = false

redis_node_type = "cache.t4g.micro"

# Run ECS tasks in public subnets to avoid NAT Gateway fixed cost.
ecs_use_public_subnets   = true
enable_nat_gateway       = false
single_nat_gateway       = true
enable_redis             = false
indexer_use_fargate_spot = true

# Switch to ARM64 only after publishing multi-arch images and verifying startup.
cpu_architecture = "X86_64"