use error::Result;
#[cfg_attr(test, double)]
#[cfg(windows)]
use eventlog::EventLog;
#[cfg(test)]
use mockall_double::double;
use std::io;
use std::{ffi::OsStr, fmt::Debug, sync::Mutex};
use tracing::{span, Metadata, Subscriber};
use tracing_core::{Event, Field};
use tracing_subscriber::fmt::format::{Compact, DefaultFields, Format, Pretty};
use tracing_subscriber::fmt::{FormatEvent, FormatFields, Layer, MakeWriter};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use widestring::WideCString;

#[cfg(windows)]
mod eventlog;
mod eventmsgs;
mod registry;
pub use self::registry::platform::*;

pub mod error;

thread_local! {
    static BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(256));
}

pub struct EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    #[cfg(windows)]
    event_log: EventLog,
    inner: Layer<S, N, F, MemWriter>,
}

impl<S, N, F> EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    pub fn new<T: Into<String> + 'static>(source: T, inner: Layer<S, N, F>) -> Result<Self> {
        #[cfg(windows)]
        let event_log = EventLog::new(source)?;

        let inner = inner.with_writer(MemWriter {});
        Ok(Self {
            inner,
            #[cfg(windows)]
            event_log,
        })
    }

    #[cfg(all(windows, test))]
    fn from_event_log(event_log: EventLog, inner: Layer<S, N, F>) -> Self {
        let inner = inner.with_writer(MemWriter {});
        Self { inner, event_log }
    }
}

impl<S> EventLogLayer<S, Pretty, Format<Pretty, ()>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn pretty<T: Into<String> + 'static>(source: T) -> Result<Self> {
        Self::new(
            source,
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_ansi(false)
                .without_time()
                .with_level(false),
        )
    }
}

impl<S> EventLogLayer<S, DefaultFields, Format<Compact, ()>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn compact<T: Into<String> + 'static>(source: T) -> Result<Self> {
        Self::new(
            source,
            tracing_subscriber::fmt::layer()
                .compact()
                .with_ansi(false)
                .without_time()
                .with_level(false),
        )
    }
}

impl<S, N, F> tracing_subscriber::Layer<S> for EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N> + 'static,
{
    #[cfg(windows)]
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.inner.enabled(metadata, ctx)
    }

    #[cfg(not(windows))]
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        false
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        self.inner.on_new_span(attrs, id, ctx)
    }

    fn on_record(
        &self,
        span: &tracing_core::span::Id,
        values: &tracing_core::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        self.inner.on_record(span, values, ctx)
    }

    #[cfg(windows)]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        self.inner.on_event(event, ctx);

        let mut category = "".to_owned();
        let mut visitor = |field: &Field, value: &dyn Debug| {
            if field.name() == "category" {
                category = format!("{:?}", value);
            }
        };
        event.record(&mut visitor);
        let field = BUFFER.with(|buffer| {
            let mut data = buffer.lock().unwrap();
            let str_data = std::str::from_utf8(data.as_slice()).unwrap();
            let c_str = WideCString::from_os_str(OsStr::new(str_data)).unwrap();
            data.clear();
            c_str
        });

        self.event_log
            .report_event(
                *event.metadata().level(),
                eventmsgs::get_category(category),
                field,
            )
            .unwrap();
    }

    fn on_enter(&self, id: &tracing_core::span::Id, ctx: Context<'_, S>) {
        self.inner.on_enter(id, ctx)
    }

    fn on_exit(&self, id: &tracing_core::span::Id, ctx: Context<'_, S>) {
        self.inner.on_exit(id, ctx);
    }

    fn on_close(&self, id: tracing_core::span::Id, ctx: Context<'_, S>) {
        self.inner.on_close(id, ctx)
    }
}

struct MemWriter;

impl std::io::Write for MemWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        BUFFER.with(|buffer| {
            buffer.lock().unwrap().extend_from_slice(buf);
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for MemWriter {
    type Writer = MemWriter;

    fn make_writer(&'a self) -> Self::Writer {
        MemWriter {}
    }
}

#[cfg(test)]
#[path = "./lib_test.rs"]
mod lib_test;
