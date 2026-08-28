resource "random_password" "db" {
  length  = 32
  special = false
}

resource "aws_db_subnet_group" "main" {
  name       = "${local.name}-db"
  subnet_ids = aws_subnet.private[*].id
  tags       = { Name = "${local.name}-db-subnets" }
}

resource "aws_db_instance" "main" {
  identifier                 = "${local.name}-postgres"
  engine                     = "postgres"
  engine_version             = "16"
  instance_class             = var.db_instance_class
  allocated_storage          = var.db_allocated_storage
  storage_type               = "gp3"
  db_name                    = var.db_name
  username                   = var.db_username
  password                   = random_password.db.result
  db_subnet_group_name       = aws_db_subnet_group.main.name
  vpc_security_group_ids     = [aws_security_group.rds.id]
  publicly_accessible        = false
  multi_az                   = var.environment == "production"
  backup_retention_period    = var.environment == "production" ? 7 : 1
  skip_final_snapshot        = var.environment != "production"
  deletion_protection        = var.rds_deletion_protection
  auto_minor_version_upgrade = true
  apply_immediately          = var.environment != "production"

  tags = { Name = "${local.name}-postgres" }
}

resource "aws_elasticache_subnet_group" "main" {
  count      = var.enable_redis ? 1 : 0
  name       = "${local.name}-redis"
  subnet_ids = aws_subnet.private[*].id
}

resource "aws_elasticache_replication_group" "main" {
  count                      = var.enable_redis ? 1 : 0
  replication_group_id       = "${local.name}-redis"
  description                = "StellarRoute quote cache / rate limits"
  engine                     = "redis"
  engine_version             = "7.1"
  node_type                  = var.redis_node_type
  num_cache_clusters         = 1
  port                       = 6379
  parameter_group_name       = "default.redis7"
  subnet_group_name          = aws_elasticache_subnet_group.main[0].name
  security_group_ids         = [aws_security_group.redis.id]
  at_rest_encryption_enabled = true
  # Transit encryption + AUTH require a TLS-capable redis client (rediss://).
  # Current workspace redis crate has no TLS features — keep in-VPC + SG only.
  # Re-enable transit_encryption when tls-native-tls / tls-rustls is added.
  transit_encryption_enabled = false
  automatic_failover_enabled = false
  apply_immediately          = var.environment != "production"

  tags = { Name = "${local.name}-redis" }
}

locals {
  database_url = format(
    "postgresql://%s:%s@%s:%d/%s",
    var.db_username,
    random_password.db.result,
    aws_db_instance.main.address,
    aws_db_instance.main.port,
    var.db_name,
  )

  redis_url = var.enable_redis ? format(
    "redis://%s:%d",
    aws_elasticache_replication_group.main[0].primary_endpoint_address,
    aws_elasticache_replication_group.main[0].port,
  ) : ""
}
