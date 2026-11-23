terraform {
^^^^^^^^^ required-version
  backend "s3" {}
  required_providers {
    aws = { source = "hashicorp/aws", version = "~> 5" }
    gcp = { source = "hashicorp/google" }
#   ^^^ required-providers
  }
}

resource "aws_instance" "a" {}
resource "gcp_thing" "b" {}
resource "azure_thing" "c" {}
resource "azure_thing" "d" {}
^^^^^^^^ required-providers
resource "terraform_data" "e" {}
