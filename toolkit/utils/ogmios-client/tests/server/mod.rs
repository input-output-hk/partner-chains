use jsonrpsee::{
	server::Server,
	types::{ErrorObjectOwned, Params},
	Extensions, RpcModule,
};
use serde_json::Value;
use std::net::SocketAddr;

pub async fn for_single_test<F>(method: &'static str, handler: F) -> anyhow::Result<SocketAddr>
where
	F: Fn(Params) -> Result<Value, ErrorObjectOwned> + Send + Sync + 'static,
{
	let server = Server::builder().build("127.0.0.1:0".parse::<SocketAddr>()?).await?;
	let mut module = RpcModule::new(());
	module.register_method(method, move |params: Params, _ctx: &(), _e: &Extensions| {
		handler(params)
	})?;
	let addr = server.local_addr()?;
	let handle = server.start(module);
	// It will stop when test main exists.
	tokio::spawn(handle.stopped());
	Ok(addr)
}
