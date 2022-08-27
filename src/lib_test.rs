use super::*;
use tracing::{info, Level};
use tracing_subscriber::layer::SubscriberExt;

#[test]
fn test() {
    let mut event_log = EventLog::default();
    event_log
        .expect_report_event()
        .returning(|_: Level, _, _| Ok(()))
        .once();
    let layer = EventLogLayer::from_event_log(event_log, tracing_subscriber::fmt::layer().pretty());

    let reg = tracing_subscriber::registry().with(layer);
    let _guard = tracing::subscriber::set_default(reg);
    info!("test log");
}