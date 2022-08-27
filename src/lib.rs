use registry::{Data, Hive, Security};
use std::io;
use std::sync::Arc;
use std::{ffi::OsStr, fmt::Debug, ptr::null_mut, sync::Mutex};
use tracing::{span, Level, Metadata, Subscriber};
use tracing_core::{Event, Field};
use tracing_subscriber::fmt::format::{Compact, DefaultFields, Format, Pretty};
use tracing_subscriber::fmt::{FormatEvent, FormatFields, Layer, MakeWriter};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use widestring::WideCString;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::PSID,
        System::EventLog::{
            self, EventSourceHandle, EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE,
            EVENTLOG_WARNING_TYPE,
        },
    },
};

pub mod eventmsgs;

pub struct EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    _source: WideCString,
    event_source_handle: EventSourceHandle,
    data: Arc<Mutex<Vec<u8>>>,
    inner: Layer<S, N, F, MemWriter>,
}

impl<S, N, F> EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    pub fn new(source: impl Into<String>, inner: Layer<S, N, F>) -> Self {
        let source = WideCString::from_os_str(source.into()).unwrap();

        let event_source_handle = unsafe {
            EventLog::RegisterEventSourceW(PCWSTR::null(), PCWSTR::from_raw(source.as_ptr()))
                .unwrap()
        };
        let data = Arc::new(Mutex::new(vec![]));
        let inner = inner.with_writer(MemWriter(data.clone()));
        Self {
            inner,
            data,
            _source: source,
            event_source_handle,
        }
    }
}

impl<S> EventLogLayer<S, Pretty, Format<Pretty, ()>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn pretty(source: impl Into<String>) -> Self {
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
    pub fn compact(source: impl Into<String>) -> Self {
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

impl<S, N, F> Drop for EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    fn drop(&mut self) {
        unsafe { EventLog::DeregisterEventSource(self.event_source_handle) };
    }
}

impl<S, N, F> tracing_subscriber::Layer<S> for EventLogLayer<S, N, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N> + 'static,
{
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.inner.enabled(metadata, ctx)
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

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        self.inner.on_event(event, ctx);

        let mut category = "".to_owned();
        let mut visitor = |field: &Field, value: &dyn Debug| {
            if field.name() == "category" {
                category = format!("{:?}", value);
            }
        };
        event.record(&mut visitor);
        let (msg_type, level) = match *event.metadata().level() {
            Level::ERROR => (EVENTLOG_ERROR_TYPE, eventmsgs::MSG_ERROR),
            Level::WARN => (EVENTLOG_WARNING_TYPE, eventmsgs::MSG_WARNING),
            Level::INFO => (EVENTLOG_INFORMATION_TYPE, eventmsgs::MSG_INFO),
            Level::DEBUG => (EVENTLOG_INFORMATION_TYPE, eventmsgs::MSG_DEBUG),
            Level::TRACE => (EVENTLOG_INFORMATION_TYPE, eventmsgs::MSG_TRACE),
        };
        unsafe {
            let mut data = self.data.lock().unwrap();
            let info = String::from_utf8(data.clone()).unwrap();
            data.clear();

            let mut fields_vec = vec![WideCString::from_os_str(OsStr::new(&info)).unwrap()];
            let pwstrs = fields_vec
                .iter_mut()
                .map(|f| windows::core::PWSTR::from_raw(f.as_mut_ptr()))
                .collect::<Vec<_>>();

            let res = EventLog::ReportEventW(
                self.event_source_handle,
                msg_type,
                eventmsgs::get_category(category),
                level,
                PSID(null_mut()),
                0,
                &pwstrs,
                null_mut(),
            );
        }
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

struct MemWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for MemWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for MemWriter {
    type Writer = MemWriter;

    fn make_writer(&'a self) -> Self::Writer {
        MemWriter(self.0.clone())
    }
}

const REG_BASEKEY: &str = r"SYSTEM\CurrentControlSet\Services\EventLog\Application";

pub fn register(name: &str) {
    let current_exe = std::env::current_exe().unwrap();
    let exe_path = current_exe.to_str().unwrap();
    println!("exe path {exe_path}");
    let exe_path = &exe_path.replacen("\\\\?\\", "", 1);
    let key = Hive::LocalMachine
        .open(REG_BASEKEY, Security::Write)
        .unwrap();
    let app_key = key.create(name, Security::Write).unwrap();
    app_key
        .set_value(
            "EventMessageFile",
            &Data::String(exe_path.try_into().unwrap()),
        )
        .unwrap();
    app_key
        .set_value(
            "CategoryMessageFile",
            &Data::String(exe_path.try_into().unwrap()),
        )
        .unwrap();
    app_key
        .set_value(
            "ParameterMessageFile",
            &Data::String(exe_path.try_into().unwrap()),
        )
        .unwrap();
    app_key
        .set_value("CategoryCount", &Data::U32(eventmsgs::CATEGORY_COUNT))
        .unwrap();
    app_key
        .set_value("TypesSupported", &Data::U32(7u32))
        .unwrap();
}

pub fn deregister(name: &str) {
    let key = Hive::LocalMachine
        .open(REG_BASEKEY, Security::Read)
        .unwrap();
    key.delete(name, true).unwrap();
}
