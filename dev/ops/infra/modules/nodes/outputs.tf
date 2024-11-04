output "ip" {
  value       = resource.aws_instance.node.public_ip
  description = "The public IP of the instance"
}
