resource "aws_secretsmanager_secret" "app" {
  name                    = "${local.name}/app"
  description             = "StellarRoute runtime secrets (DB, Redis, Horizon, Soroban, router, auth)"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = { Name = "${local.name}-app-secrets" }
}

# Seed all keys ECS expects (jsonKey injection fails if a key is missing).
# After apply, replace REPLACE_ME / change-me values via put-secret-value.
# lifecycle.ignore_changes keeps operator updates from being overwritten.
resource "aws_secretsmanager_secret_version" "app" {
  secret_id = aws_secretsmanager_secret.app.id
  secret_string = jsonencode({
    DATABASE_URL            = local.database_url
    REDIS_URL               = local.redis_url
    STELLAR_HORIZON_URL     = "https://horizon-testnet.stellar.org"
    SOROBAN_RPC_URL         = "https://soroban-testnet.stellar.org"
    ROUTER_CONTRACT_ADDRESS = "REPLACE_ME"
    AMM_POOLS               = ""
    ADMIN_AUTH_TOKEN        = "change-me-rotate-before-public"
    CORS_ALLOWED_ORIGINS    = "https://www.stellarroute.app,https://stellarroute.app,https://stellarroute-frontend.vercel.app"
    PUBLIC_GET_ROUTES       = "/api/v1/quote,/api/v1/pairs,/api/v1/markets,/api/v1/orderbook,/api/v1/routes,/api/v1/price-history,/health"
    RUST_LOG                = "info,stellarroute_api=info,stellarroute_indexer=info"
  })

  lifecycle {
    ignore_changes = [secret_string]
  }
}
