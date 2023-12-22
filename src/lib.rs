#![warn(clippy::cast_possible_truncation)]
#![warn(clippy::exit)]
#![cfg_attr(not(test), warn(clippy::expect_used))]
#![warn(clippy::fallible_impl_from)]
#![cfg_attr(not(test), warn(clippy::index_refutable_slice))]
#![cfg_attr(not(test), warn(clippy::indexing_slicing))]
#![cfg_attr(not(test), warn(clippy::integer_arithmetic))]
#![cfg_attr(not(test), warn(clippy::missing_panics_doc))]
#![cfg_attr(not(test), warn(clippy::panic))]
#![warn(clippy::unchecked_duration_subtraction)]
#![cfg_attr(not(test), warn(clippy::unreachable))]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

use tracing_subscriber::{prelude::*, util::SubscriberInitExt};

pub use opentelemetry::global::shutdown_tracer_provider;

pub struct DropHandler(bool);

impl Drop for DropHandler {
    fn drop(&mut self) {
        if self.0 {
            shutdown_tracer_provider();
        }
    }
}

pub fn init(
    project_name: &str,
    stdout: bool,
    file: bool,
    opentelemetry: bool,
) -> Result<DropHandler, opentelemetry::trace::TraceError> {
    if stdout || file || opentelemetry {
        let stdout_layer = if stdout {
            let owned_project_name = project_name.to_owned();
            Some(tracing_subscriber::fmt::layer().pretty().with_filter(
                tracing_subscriber::filter::filter_fn(
                    move |metadata| match metadata.module_path() {
                        Some(module_path) => {
                            module_path.starts_with(&owned_project_name.to_owned())
                                && *metadata.level() >= tracing::Level::INFO
                        }
                        None => false,
                    },
                ),
            ))
        } else {
            None
        };
        let file_layer = if file {
            let rolling_file_appender = tracing_appender::rolling::RollingFileAppender::new(
                tracing_appender::rolling::Rotation::DAILY,
                "./logs",
                format!("log_{project_name}_"),
            );
            Some(tracing_subscriber::fmt::layer().with_writer(rolling_file_appender))
        } else {
            None
        };
        let opentelemetry_layer =
            if opentelemetry {
                let tracer = opentelemetry_jaeger::new_collector_pipeline()
                    .with_endpoint("http://192.168.1.100:14268/api/traces")
                    .with_reqwest()
                    .with_service_name(project_name)
                    .install_batch(opentelemetry::runtime::Tokio)?;

                let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

                let owned_project_name = project_name.to_owned();
                Some(telemetry.with_filter(tracing_subscriber::filter::filter_fn(
                    move |metadata| match metadata.module_path() {
                        Some(module_path) => {
                            module_path.starts_with(&owned_project_name.to_owned())
                        }
                        None => false,
                    },
                )))
            } else {
                None
            };
        tracing_subscriber::Registry::default()
            .with(stdout_layer)
            .with(file_layer)
            .with(opentelemetry_layer)
            .init();
        Ok(DropHandler(opentelemetry))
    } else {
        Ok(DropHandler(false))
    }
}
