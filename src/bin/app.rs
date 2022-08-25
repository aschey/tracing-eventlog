use tracing::info;
use tracing_eventlog::{register, EventLogSubscriber};

fn main() {
    let log_source = format!("MyEventProvider2");
    register(&log_source);

    let sub = EventLogSubscriber::new(&log_source);
    tracing::subscriber::set_global_default(sub);
    info!("test");
}
