use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry_sdk::resource::{EnvResourceDetector, TelemetryResourceDetector};
use opentelemetry_sdk::Resource;

/// Initialize the open-telemetry resource.
pub fn init_resource() -> Resource {
    let detector_resources = Box::new(Resource::from_detectors(
        Duration::from_secs(10),
        vec![
            Box::new(EnvResourceDetector::new()),
            Box::new(TelemetryResourceDetector),
        ],
    ));

    Resource::new(vec![
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            "sticker-export-bot",
        ),
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
            env!("CARGO_PKG_VERSION"),
        ),
    ])
    .merge(detector_resources)
}
