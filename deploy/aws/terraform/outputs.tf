output "vpc_id" {
  value = aws_vpc.main.id
}

output "alb_dns_name" {
  description = "Point api.<domain> CNAME/alias here."
  value       = aws_lb.api.dns_name
}

output "alb_zone_id" {
  value = aws_lb.api.zone_id
}

output "api_url_http" {
  value = "http://${aws_lb.api.dns_name}"
}

output "ecr_api_repository_url" {
  value = aws_ecr_repository.api.repository_url
}

output "ecr_indexer_repository_url" {
  value = aws_ecr_repository.indexer.repository_url
}

output "ecs_cluster_name" {
  value = aws_ecs_cluster.main.name
}

output "api_service_name" {
  value = aws_ecs_service.api.name
}

output "indexer_service_name" {
  value = aws_ecs_service.indexer.name
}

output "secrets_arn" {
  value = aws_secretsmanager_secret.app.arn
}

output "secrets_name" {
  value = aws_secretsmanager_secret.app.name
}

output "database_url" {
  description = "Postgres URL (sensitive). Prefer reading from Secrets Manager."
  value       = local.database_url
  sensitive   = true
}

output "redis_url" {
  description = "Redis TLS URL (sensitive)."
  value       = local.redis_url
  sensitive   = true
}

output "rds_endpoint" {
  value = aws_db_instance.main.address
}

output "redis_endpoint" {
  value = var.enable_redis ? aws_elasticache_replication_group.main[0].primary_endpoint_address : null
}

output "cloudwatch_api_log_group" {
  value = aws_cloudwatch_log_group.api.name
}

output "cloudwatch_indexer_log_group" {
  value = aws_cloudwatch_log_group.indexer.name
}
