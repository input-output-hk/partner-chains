locals {
  region = basename(get_original_terragrunt_dir())
}

include "root" {
  path           = find_in_parent_folders()
  merge_strategy = "deep"
}

terraform {
  # See: https://github.com/gruntwork-io/terragrunt/issues/1675
  source = "${get_parent_terragrunt_dir()}/modules/nodes//."
}

generate "provider" {
  path      = "provider.tf"
  if_exists = "overwrite_terragrunt"
  contents  = <<EOF
provider "aws" {
  region = "${local.region}"
}
EOF
}



inputs = {
  volume_size   = 120
  instance_type = "r5a.xlarge"
  ssh_pub_key   = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFAMZ8ff9gmamy1zCwPCjd7nO+sr/S02xGVruG1sSP3w nrd@silva"
  label = {
    environment = local.region
    attributes  = ["public"]
  }
}
