#[cfg(windows)]
mod tests {

    use tracing::info;
    use tracing_eventlog::{register, EventLogLayer};
    use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

    #[test]
    fn super_test() {
        let source = "tracing_eventlog_test";
        register(source).unwrap();

        let layer = EventLogLayer::pretty(source).unwrap();

        let reg = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(reg);

        info!("test log");
    }
}
