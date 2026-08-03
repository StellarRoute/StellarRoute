resource "aws_ecs_cluster" "main" {
  name = local.name

  setting {
    name  = "containerInsights"
    value = var.environment == "production" ? "enabled" : "disabled"
  }

  tags = { Name = local.name }
}

# FARGATE_SPOT is opt-in per service via indexer_use_fargate_spot; always
# registering both providers here keeps the cluster ready without cost impact.
resource "aws_ecs_cluster_capacity_providers" "main" {
  cluster_name       = aws_ecs_cluster.main.name
  capacity_providers = ["FARGATE", "FARGATE_SPOT"]

  default_capacity_provider_strategy {
    capacity_provider = "FARGATE"
    weight            = 1
  }
}

locals {
  secret_arn = aws_secretsmanager_secret.app.arn

  # Keys pulled from Secrets Manager JSON into container env
  shared_secret_keys = [
    "DATABASE_URL",
    "REDIS_URL",
    "STELLAR_HORIZON_URL",
    "SOROBAN_RPC_URL",
    "ROUTER_CONTRACT_ADDRESS",
    "RUST_LOG",
  ]

  api_secret_keys = concat(local.shared_secret_keys, [
    "ADMIN_AUTH_TOKEN",
    "CORS_ALLOWED_ORIGINS",
    "PUBLIC_GET_ROUTES",
  ])

  indexer_secret_keys = concat(local.shared_secret_keys, [
    "AMM_POOLS",
  ])

  api_secrets = [
    for key in local.api_secret_keys : {
      name      = key
      valueFrom = "${local.secret_arn}:${key}::"
    }
  ]

  indexer_secrets = [
    for key in local.indexer_secret_keys : {
      name      = key
      valueFrom = "${local.secret_arn}:${key}::"
    }
  ]
}

resource "aws_ecs_task_definition" "api" {
  family                   = "${local.name}-api"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.api_cpu
  memory                   = var.api_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = var.cpu_architecture
  }

  container_definitions = jsonencode([
    {
      name      = "api"
      image     = "${aws_ecr_repository.api.repository_url}:${var.api_image_tag}"
      essential = true
      portMappings = [{
        containerPort = 8080
        protocol      = "tcp"
      }]
      environment = [
        { name = "PORT", value = "8080" },
        { name = "STELLARROUTE_ENV", value = "production" },
        { name = "ENABLE_ADMIN_ROUTES", value = "false" },
      ]
      secrets = local.api_secrets
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.api.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "api"
        }
      }
      healthCheck = {
        command     = ["CMD-SHELL", "curl -sf http://127.0.0.1:8080/health/deps || exit 1"]
        interval    = 30
        timeout     = 5
        retries     = 3
        startPeriod = 60
      }
    }
  ])
}

resource "aws_ecs_task_definition" "indexer" {
  family                   = "${local.name}-indexer"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.indexer_cpu
  memory                   = var.indexer_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = var.cpu_architecture
  }

  container_definitions = jsonencode([
    {
      name      = "indexer"
      image     = "${aws_ecr_repository.indexer.repository_url}:${var.indexer_image_tag}"
      essential = true
      environment = [
        { name = "STELLARROUTE_ENV", value = "production" },
      ]
      secrets = local.indexer_secrets
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.indexer.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "indexer"
        }
      }
    }
  ])
}

resource "aws_ecs_service" "api" {
  name            = "${local.name}-api"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = var.api_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.ecs_use_public_subnets ? aws_subnet.public[*].id : aws_subnet.private[*].id
    security_groups  = [aws_security_group.ecs.id]
    assign_public_ip = var.ecs_use_public_subnets
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = 8080
  }

  deployment_minimum_healthy_percent = 50
  deployment_maximum_percent         = 200

  depends_on = [
    aws_lb_listener.http,
    aws_secretsmanager_secret_version.app,
  ]

  tags = { Name = "${local.name}-api" }
}

resource "aws_ecs_service" "indexer" {
  name            = "${local.name}-indexer"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.indexer.arn
  desired_count   = var.indexer_desired_count
  launch_type     = var.indexer_use_fargate_spot ? null : "FARGATE"

  dynamic "capacity_provider_strategy" {
    for_each = var.indexer_use_fargate_spot ? [1] : []
    content {
      capacity_provider = "FARGATE_SPOT"
      weight            = 100
    }
  }

  network_configuration {
    subnets          = var.ecs_use_public_subnets ? aws_subnet.public[*].id : aws_subnet.private[*].id
    security_groups  = [aws_security_group.ecs.id]
    assign_public_ip = var.ecs_use_public_subnets
  }

  deployment_minimum_healthy_percent = 0
  deployment_maximum_percent         = 100

  depends_on = [
    aws_secretsmanager_secret_version.app,
    aws_ecs_cluster_capacity_providers.main,
  ]

  tags = { Name = "${local.name}-indexer" }
}
