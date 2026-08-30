resource "aws_cloudwatch_log_group" "api" {
  name              = "/ecs/${local.name}/api"
  retention_in_days = var.environment == "production" ? 30 : 14
}

resource "aws_cloudwatch_log_group" "indexer" {
  name              = "/ecs/${local.name}/indexer"
  retention_in_days = var.environment == "production" ? 30 : 14
}
