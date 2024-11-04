variable "label" {
  type = object({
    namespace   = optional(string)
    environment = optional(string)
    stage       = optional(string)
    name        = optional(string)
    attributes  = optional(list(string))
    delimiter   = optional(string)
    tags        = optional(map(string))
  })
  description = "The label to use for this module"
}

variable "volume_size" {
  type        = number
  description = "The size of the mounted volume"
}

variable "instance_type" {
  type        = string
  description = "Instance type to use"
}

variable "ssh_pub_key" {
  type        = string
  description = "The ssh public key to connect to the instance"
}
