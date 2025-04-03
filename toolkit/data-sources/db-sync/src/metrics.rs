use log::warn;
use substrate_prometheus_endpoint::{
	register, CounterVec, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64,
};

#[derive(Clone)]
pub struct McFollowerMetrics {
	time_elapsed: HistogramVec,
	call_count: CounterVec<U64>,
}

impl McFollowerMetrics {
	pub fn time_elapsed(&self) -> &HistogramVec {
		&self.time_elapsed
	}
	pub fn call_count(&self) -> &CounterVec<U64> {
		&self.call_count
	}
	pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			time_elapsed: register(
				HistogramVec::new(
					HistogramOpts::new(
						"partner_chains_data_source_method_time_elapsed",
						"Time spent in a method call",
					),
					&["method_name"],
				)?,
				registry,
			)?,
			call_count: register(
				CounterVec::new(
					Opts::new(
						"partner_chains_data_source_method_call_count",
						"Total number of data source method calls",
					),
					&["method_name"],
				)?,
				registry,
			)?,
		})
	}
}

pub fn register_metrics_warn_errors(
	metrics_registry_opt: Option<&Registry>,
) -> Option<McFollowerMetrics> {
	metrics_registry_opt.and_then(|registry| match McFollowerMetrics::register(registry) {
		Ok(metrics) => Some(metrics),
		Err(err) => {
			warn!("Failed registering data source metrics with err: {}", err);
			None
		},
	})
}

/// Logs each method invocation and each returned result.
/// Has to be made at the level of trait, because otherwise #[async_trait] is expanded first.
/// '&self' matching yields "__self" identifier not found error, so "&$self:tt" is required.
/// Works only if return type is Result.
#[macro_export]
macro_rules! observed_async_trait {
	(impl $trait_name:ident for $target_type:ty {
		$(type $type_name:ident = $type:ty;)*
		$(async fn $method:ident(&$self:tt $(,$param_name:ident: $param_type:ty)* $(,)?) -> $res:ty $body:block)*
	})=> {
		#[async_trait::async_trait]
		impl $trait_name for $target_type {
		$(type $type_name = $type;)*
		$(
			async fn $method(&$self $(,$param_name: $param_type)*,) -> $res {
				let method_name = stringify!($method);
				let _timer = if let Some(metrics) = &$self.metrics_opt {
					metrics.call_count().with_label_values(&[method_name]).inc();
					Some(metrics.time_elapsed().with_label_values(&[method_name]).start_timer())
				} else { None };
				let params: Vec<String> = vec![$(format!("{:?}", $param_name.clone()),)*];
				log::debug!("{} called with parameters: {:?}", method_name, params);
				let result = $body;
				match &result {
					Ok(value) => {
						log::debug!("{} returns {:?}", method_name, value);
					},
					Err(error) => {
						log::error!("{} failed with {:?}", method_name, error);
					},
				};
				result
			}
		)*
		}
	};
}

#[cfg(test)]
pub mod mock {
	use crate::metrics::McFollowerMetrics;
	use substrate_prometheus_endpoint::{CounterVec, HistogramOpts, HistogramVec, Opts};

	pub fn test_metrics() -> McFollowerMetrics {
		McFollowerMetrics {
			time_elapsed: HistogramVec::new(HistogramOpts::new("test", "test"), &["method_name"])
				.unwrap(),
			call_count: CounterVec::new(Opts::new("test", "test"), &["method_name"]).unwrap(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::metrics::{mock::test_metrics, McFollowerMetrics};
	use async_trait::async_trait;
	use std::convert::Infallible;
	use substrate_prometheus_endpoint::prometheus::core::Metric;

	struct MetricsMacroTestStruct {
		metrics_opt: Option<McFollowerMetrics>,
	}

	#[async_trait]
	trait MetricMacroTestTrait {
		async fn test_method_one(&self) -> Result<(), Infallible>;
		async fn test_method_two(&self) -> Result<(), Infallible>;
	}

	observed_async_trait!(
	impl MetricMacroTestTrait for MetricsMacroTestStruct {
		async fn test_method_one(&self) -> Result<(), Infallible> {
			tokio::time::sleep(core::time::Duration::from_millis(10)).await;
			Ok(())
		}

		async fn test_method_two(&self) -> Result<(), Infallible> {
			Ok(())
		}
	});

	#[tokio::test]
	async fn calculate_metrics_correctly() {
		let metrics = test_metrics();
		let histogram_method_one = metrics.time_elapsed().with_label_values(&["test_method_one"]);
		let histogram_method_two = metrics.time_elapsed().with_label_values(&["test_method_two"]);
		let histogram_method_random = metrics.time_elapsed().with_label_values(&["random"]);
		let counter_method_one = metrics.call_count().with_label_values(&["test_method_one"]);
		let counter_method_two = metrics.call_count().with_label_values(&["test_method_two"]);
		let counter_method_random = metrics.call_count().with_label_values(&["random"]);

		let metrics_struct = MetricsMacroTestStruct { metrics_opt: Some(metrics.clone()) };
		metrics_struct.test_method_one().await.unwrap();
		metrics_struct.test_method_two().await.unwrap();

		for bucket in histogram_method_one.metric().get_histogram().get_bucket().iter().take(2) {
			assert_eq!(bucket.get_cumulative_count(), 0);
		}

		// Assert below has a tiny potential to be flaky - if it is, please increase sleep time in MetricMacroTestTrait implementation or
		// remove the Assert completely
		assert!(histogram_method_one.get_sample_sum() > histogram_method_two.get_sample_sum());
		assert_eq!(histogram_method_one.get_sample_count(), 1);
		assert_eq!(histogram_method_two.get_sample_count(), 1);
		assert_eq!(histogram_method_random.get_sample_count(), 0);

		assert_eq!(counter_method_one.get(), 1);
		assert_eq!(counter_method_two.get(), 1);
		assert_eq!(counter_method_random.get(), 0);
	}
}
