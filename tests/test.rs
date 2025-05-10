#[cfg(windows)]
mod tests {
    use std::process::Command;
    use tracing::info;
    use tracing_eventlog::EventLogRegistry;
    use tracing_eventlog::{EventLogLayer, LogSource};
    use tracing_subscriber::prelude::*;

    #[test]
    fn test_application() {
        let start = chrono::Local::now()
            .format("%d %b %Y %I:%M:%S %p %:z")
            .to_string();
        let source = LogSource::application("Tracing Application");
        source.register().unwrap();

        let layer = EventLogLayer::pretty("Tracing Application").unwrap();

        let reg = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(reg);
        let log = format!("basic test log {start}");
        info!(log);
        verify_log("Application", "Tracing Application", &start, &log);
        source.deregister().unwrap();
    }

    #[test]
    fn test_custom() {
        let start = chrono::Local::now()
            .format("%d %b %Y %I:%M:%S %p %:z")
            .to_string();
        let source = LogSource::custom("Custom Tracing Application", None);
        source.register().unwrap();

        let layer = EventLogLayer::pretty("Custom Tracing Application").unwrap();

        let reg = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(reg);
        let log = format!("custom test log {start}");
        info!(log);
        verify_log(
            "Custom Tracing Application",
            "Custom Tracing Application",
            &start,
            &log,
        );
        source.deregister().unwrap();
    }

    fn verify_log(log_source: &str, log_name: &str, start_time: &str, log_msg: &str) {
        let mut command = Command::new("powershell");
        command.arg("-Command").arg(format!(
            "Get-WinEvent -FilterHashtable @{{
                Logname='{log_source}'
                ProviderName='{log_name}'
                StartTime=[datetime]::parseexact('{start_time}', 'dd MMM yyyy hh:mm:ss tt zzz', $null)
            }} | Format-Table -HideTableHeaders LevelDisplayName, Message",
        ));
        let out = command.output().unwrap().stdout;
        let out_str = String::from_utf8(out).unwrap();
        let trimmed = out_str.split_whitespace().collect::<Vec<_>>().join(" ");
        // Using "ends_with" here because sometimes there's an "Information" prefix but it's not consistent
        assert!(trimmed.ends_with(&format!("test::tests: log: \"{log_msg}\"...")));
    }
}
