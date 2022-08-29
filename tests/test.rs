#[cfg(windows)]
mod tests {

    use tracing::info;
    use tracing_eventlog::EventLogRegistry;
    use tracing_eventlog::{EventLogLayer, LogSource};
    use tracing_subscriber::prelude::*;

    #[test]
    fn super_test() {
        let source = LogSource::custom("Tracing EventLog", None);
        source.register().unwrap();

        let layer = EventLogLayer::pretty("Tracing EventLog").unwrap();

        let reg = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(reg);

        info!("test log");
    }
}
