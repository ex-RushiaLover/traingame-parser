use tracing::Level;
use tracing_subscriber::{
    EnvFilter, Layer, filter, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

pub fn init(level: tracing::Level) {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(level.into())
                .from_env()
                .unwrap()
                .add_directive("ureq=error".parse().unwrap()),
        )
        .without_time()
        .with_target(false)
        .init();
}

pub fn init_info_only() {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .without_time()
                .with_target(false)
                .with_filter(filter::filter_fn(|m| m.level() == &Level::INFO)),
        )
        .init();
}
