# Getting started

This is a guide for partner chains developers to get started in the project. When you complete it, you should be able to build the project, test locally, and access development environment hosted in EKS.

Prerequisites:
* `nix` installed
* `docker` installed
* `protoc` installed [link](https://google.github.io/proto-lens/installing-protoc.html)

## Nix

The development shells use Nix with flakes enabled. It should provide the most
important tools (besides `docker`), to work in this project: `cargo` for
building the project, and `aws` and `kubectl` to interact with the development
environment.

[Install a recent version of Nix](https://zero-to-nix.com/start/install), and
use it to enter the project shell, preferably with [`direnv`](#direnv-support),
but `nix develop` will also work.

It is recommended to add at least the following snippet to your
`/etc/nix/nix.conf` for all methods:
```
extra-experimental-features = nix-command flakes
```


Move to the project's directory and allow direnv to read the `.envrc` file:
```
# if you need to enable direnv on the project for the first time
direnv allow

# or reload it if the nix content has changed
direnv reload
```

Build the project, to verify Rust toolchain is available:
```
cargo build
```


## Direnv support

For those using [direnv](https://direnv.net/), you may choose to use it to enter
the Nix shell since it not only allows you to use your shell of choice, but it
will also load the environmental variables used in development of this project.

It is highly recommended that you use this route not only for the
above-mentioned benefits, but because it will allow your shell to survive a
garbage collection, making entering it super quick after the initial build.
