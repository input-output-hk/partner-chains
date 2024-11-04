data "aws_vpc" "default" {
  default = true
}

module "label" {
  # v0.25.0
  source = "github.com/cloudposse/terraform-null-label?ref=488ab91e34a24a86957e397d9f7262ec5925586a"

  namespace   = var.label.namespace
  environment = var.label.environment
  stage       = var.label.stage
  name        = var.label.name
  attributes  = var.label.attributes
  delimiter   = var.label.delimiter
  tags        = var.label.tags
}

data "aws_subnets" "default" {
  filter {
    name   = "vpc-id"
    values = [data.aws_vpc.default.id]
  }
}

data "aws_ami" "nixos" {
  most_recent = true
  owners      = ["080433136561"] # NixOS's AWS account ID

  filter {
    name   = "name"
    values = ["NixOS-*"]
  }

  filter {
    name   = "state"
    values = ["available"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }

  filter {
    name   = "root-device-type"
    values = ["ebs"]
  }

  filter {
    name   = "architecture"
    values = ["x86_64"]
  }
}

resource "aws_key_pair" "node" {
  key_name   = "node-key"
  public_key = var.ssh_pub_key

}

resource "aws_security_group" "node" {
  name   = module.label.id
  vpc_id = data.aws_vpc.default.id

  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  tags = module.label.tags
}

resource "aws_instance" "node" {
  associate_public_ip_address = true
  ami                         = data.aws_ami.nixos.id
  instance_type               = var.instance_type
  key_name                    = "node-key"
  subnet_id                   = data.aws_subnets.default.ids[0]
  vpc_security_group_ids      = [aws_security_group.node.id]
  root_block_device {
    volume_size = var.volume_size
  }
  lifecycle {
    ignore_changes = [ami]
  }
  tags = module.label.tags
}
