use core::fmt;
use std::{
    ffi::{OsStr, OsString},
    ptr::null_mut,
};

use registry::{Data, Hive, Security};
use tracing_core::{field, Event};
use widestring::WideCString;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::PSID,
        System::EventLog::{self, EventSourceHandle, EVENTLOG_ERROR_TYPE},
    },
};

pub mod eventmsgs;

pub struct EventLogSubscriber {
    _source: WideCString,
    event_source_handle: EventSourceHandle,
}

impl EventLogSubscriber {
    pub fn new(source: impl Into<OsString>) -> Self {
        let source = WideCString::from_os_str(source.into()).unwrap();

        let event_source_handle = unsafe {
            EventLog::RegisterEventSourceW(PCWSTR::null(), PCWSTR::from_raw(source.as_ptr()))
                .unwrap()
        };
        Self {
            _source: source,
            event_source_handle,
        }
    }
}

impl Drop for EventLogSubscriber {
    fn drop(&mut self) {
        unsafe { EventLog::DeregisterEventSource(self.event_source_handle) };
    }
}

impl tracing_core::subscriber::Subscriber for EventLogSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &tracing_core::span::Attributes<'_>) -> tracing_core::span::Id {
        println!("new span");
        tracing_core::span::Id::from_u64(1)
    }

    fn record(&self, span: &tracing_core::span::Id, values: &tracing_core::span::Record<'_>) {
        println!("record");
    }

    fn record_follows_from(&self, span: &tracing_core::span::Id, follows: &tracing_core::span::Id) {
        println!("record follows from");
    }

    fn event(&self, event: &Event<'_>) {
        let mut fields_vec: Vec<WideCString> = vec![];
        let mut format_fields = |field: &field::Field, value: &dyn fmt::Debug| {
            println!("{}={:?}", field.name(), value);
            let entry =
                WideCString::from_os_str(OsStr::new(&format!("{}={:?}", field.name(), value)))
                    .unwrap();

            fields_vec.push(entry);
        };

        event.record(&mut format_fields);

        let pwstrs = fields_vec
            .iter_mut()
            .map(|f| windows::core::PWSTR::from_raw(f.as_mut_ptr()))
            .collect::<Vec<_>>();
        unsafe {
            println!("{}", std::io::Error::last_os_error());

            let res = EventLog::ReportEventW(
                self.event_source_handle,
                EVENTLOG_ERROR_TYPE,
                eventmsgs::DATABASE_CATEGORY,
                eventmsgs::MSG_ERROR,
                PSID(null_mut()),
                0,
                &pwstrs,
                null_mut(),
            );
            println!("{}", std::io::Error::last_os_error());
        }
    }

    fn enter(&self, span: &tracing_core::span::Id) {
        println!("enter");
    }

    fn exit(&self, span: &tracing_core::span::Id) {
        println!("exit");
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
        .set_value("CategoryCount", &Data::U32(3u32))
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
