terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = ">= 3.5"
    }
  }

  # Optional remote state — uncomment and configure for team use:
  # backend "s3" {
  #   bucket         = "your-tf-state-bucket"
  #   key            = "stellarroute/aws/terraform.tfstate"
  #   region         = "us-east-1"
  #   dynamodb_table = "your-tf-locks"
  #   encrypt        = true
  # }
}
