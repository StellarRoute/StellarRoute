variable "aws_region" {
  type        = string
  description = "AWS region for all resources."
  default     = "us-east-1"
}

variable "project_name" {
  type        = string
  description = "Short name used in resource names (lowercase, hyphens)."
  default     = "stellarroute"
}

variable "environment" {
  type        = string
  description = "Environment label (staging | production)."
  default     = "staging"
}

variable "vpc_cidr" {
  type        = string
  description = "VPC CIDR block."
  default     = "10.40.0.0/16"
}

variable "availability_zones" {
  type        = list(string)
  description = "AZs for subnets (use two for ALB/RDS)."
  default     = ["us-east-1a", "us-east-1b"]
}

variable "certificate_arn" {
  type        = string
  description = "ACM certificate ARN in the same region as the ALB (HTTPS). Leave empty to create HTTP-only ALB (not recommended for public staging)."
  default     = ""
}

variable "api_image_tag" {
  type        = string
  description = "ECR tag for stellarroute-api."
  default     = "latest"
}

variable "indexer_image_tag" {
  type        = string
  description = "ECR tag for stellarroute-indexer."
  default     = "latest"
}

variable "api_cpu" {
  type    = number
  default = 512
}

variable "api_memory" {
  type    = number
  default = 1024
}

variable "api_desired_count" {
  type    = number
  default = 1
}

variable "indexer_cpu" {
  type    = number
  default = 512
}

variable "indexer_memory" {
  type    = number
  default = 1024
}

variable "indexer_desired_count" {
  type        = number
  description = "Keep at 1 unless you have a multi-writer design."
  default     = 1
}

variable "db_instance_class" {
  type    = string
  default = "db.t4g.micro"
}

variable "db_allocated_storage" {
  type    = number
  default = 20
}

variable "db_name" {
  type    = string
  default = "stellarroute"
}

variable "db_username" {
  type    = string
  default = "stellarroute"
}

variable "rds_deletion_protection" {
  type        = bool
  description = "Protect RDS from terraform destroy. Disable only for disposable staging."
  default     = true
}

variable "redis_node_type" {
  type    = string
  default = "cache.t4g.micro"
}

variable "enable_nat_gateway" {
  type        = bool
  description = "Required for Fargate tasks in private subnets to reach Horizon/Soroban/ECR. Set to false (and set ecs_use_public_subnets=true) to eliminate the ~$32+/mo NAT Gateway cost entirely for staging."
  default     = true
}

variable "single_nat_gateway" {
  type        = bool
  description = "One NAT for all private subnets (cheaper staging)."
  default     = true
}

variable "ecs_use_public_subnets" {
  type        = bool
  description = "Run ECS tasks directly in public subnets with public IPs instead of NAT-routed private subnets. Security groups still block all inbound traffic except ALB->API, so this is safe for staging and removes the need for a NAT Gateway. Pair with enable_nat_gateway=false for the lowest-cost setup."
  default     = false
}

variable "indexer_use_fargate_spot" {
  type        = bool
  description = "Run the indexer task on Fargate Spot (~70% cheaper than on-demand). Safe for the indexer because it is a single, restart-safe worker; not recommended for the user-facing API service."
  default     = false
}

variable "enable_redis" {
  type        = bool
  description = "Provision ElastiCache Redis. REDIS_URL is optional at the application layer (quote caching/rate limiting degrade gracefully without it) — set to false to skip ElastiCache entirely and save its node + data-transfer cost for early staging."
  default     = true
}

variable "cpu_architecture" {
  type        = string
  description = "Fargate CPU architecture. ARM64 (Graviton) is ~20% cheaper per vCPU/GB-hour than X86_64, but requires multi-arch Docker images (docker buildx --platform linux/amd64,linux/arm64). Verify Dockerfile.api/Dockerfile.indexer build cleanly for arm64 before switching."
  default     = "X86_64"
  validation {
    condition     = contains(["X86_64", "ARM64"], var.cpu_architecture)
    error_message = "cpu_architecture must be X86_64 or ARM64."
  }
}
