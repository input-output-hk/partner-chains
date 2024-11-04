remote_state {
  backend = "s3"
  generate = {
    path      = "backend.tf"
    if_exists = "overwrite"
  }
  config = {
    bucket = "iog-pc-testnet"

    key            = "${split("envs/", path_relative_to_include())[1]}/terraform.tfstate"
    region         = "eu-central-1"
    encrypt        = true
    dynamodb_table = "iog-pc-testnet-lock"
  }
}

skip = true

inputs = {
  label = {
    namespace = "iog"
    stage     = split("/", path_relative_to_include())[1]
    name      = split("/", path_relative_to_include())[2]
  }
}
