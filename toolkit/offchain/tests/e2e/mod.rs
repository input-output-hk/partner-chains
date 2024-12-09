use std::time::Duration;
use testcontainers::{clients::Cli, GenericImage};
use tokio::time::sleep;

/// Tests only that the image is available on ghcr.io,
/// if this test fails, please build a new image using Dockerfile
/// present in this crate tests/e2e/Dockerfile
#[tokio::test]
async fn e2e_test_docker_image_available() {
	let image = GenericImage::new(
		"partner-chains-smart-contracts-tests-cardano-node-ogmios",
		"v10.2.1-v6.9.0",
	);
	let docker = Cli::default();
	docker.run(image);
	()
}

#[tokio::test]
async fn init_goveranance() {
	let image = GenericImage::new(
		"partner-chains-smart-contracts-tests-cardano-node-ogmios",
		"v10.2.1-v6.9.0",
	);
	let docker = Cli::default();
	docker.run(image);

	sleep(Duration::from_secs(300)).await;
	()
}
