use tracing::{debug, info, instrument};
use tracing_eventlog::{register, EventLogLayer};
use tracing_subscriber::{
    fmt::format::{self, Compact, DefaultFields},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
};

fn main() {
    let log_source = format!("MyEventProvider2");
    //register(&log_source);

    tracing_subscriber::registry()
        .with(EventLogLayer::new(
            log_source,
            tracing_subscriber::fmt::layer().pretty(),
        ))
        .init();
    let n = 5;
    let sequence = fibonacci_seq(n);
    info!("The first {} fibonacci numbers are {:?}", n, sequence);
}

#[instrument]
fn nth_fibonacci(n: u64) -> u64 {
    if n == 0 || n == 1 {
        debug!("Base case");
        1
    } else {
        debug!("Recursing");
        nth_fibonacci(n - 1) + nth_fibonacci(n - 2)
    }
}

#[instrument]
fn fibonacci_seq(to: u64) -> Vec<u64> {
    let mut sequence = vec![];

    for n in 0..=to {
        debug!("Pushing {n} fibonacci", n = n);
        sequence.push(nth_fibonacci(n));
    }

    sequence
}
