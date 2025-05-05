#[cfg(test)]
use mockall::automock;
use tracing::Level;
use widestring::WideCString;
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::EventLog::{
            self as WinEventLog, EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE,
            EVENTLOG_WARNING_TYPE, REPORT_EVENT_TYPE,
        },
    },
    core::PCWSTR,
};

use crate::{
    error::{EventLogError, Result},
    eventmsgs,
};

pub(crate) struct EventLog {
    handle: HANDLE,
}

// TODO: is this actually safe?
unsafe impl Send for EventLog {}

unsafe impl Sync for EventLog {}

pub(crate) struct MessageType {
    message_type: REPORT_EVENT_TYPE,
    level: u32,
}

impl From<Level> for MessageType {
    fn from(level: Level) -> Self {
        let (message_type, level) = match level {
            Level::ERROR => (EVENTLOG_ERROR_TYPE, eventmsgs::MSG_ERROR),
            Level::WARN => (EVENTLOG_WARNING_TYPE, eventmsgs::MSG_WARNING),
            Level::INFO => (EVENTLOG_INFORMATION_TYPE, eventmsgs::MSG_INFO),
            Level::DEBUG => (EVENTLOG_INFORMATION_TYPE, eventmsgs::MSG_DEBUG),
            Level::TRACE => (EVENTLOG_INFORMATION_TYPE, eventmsgs::MSG_TRACE),
        };
        Self {
            message_type,
            level,
        }
    }
}

#[cfg_attr(test, automock)]
impl EventLog {
    pub(crate) fn new<T: Into<String> + 'static>(source: T) -> Result<Self> {
        let source =
            WideCString::from_os_str(source.into()).map_err(EventLogError::StrConvertError)?;
        let win_source = PCWSTR::from_raw(source.as_ptr());
        let handle = unsafe {
            WinEventLog::RegisterEventSourceW(PCWSTR::null(), win_source)
                .map_err(EventLogError::WindowsError)?
        };
        Ok(Self { handle })
    }

    pub(crate) fn report_event<T: Into<MessageType> + 'static>(
        &self,
        message_type: T,
        category: u16,
        mut message: WideCString,
    ) -> Result<()> {
        let message_type: MessageType = message_type.into();

        let pwstrs = vec![windows::core::PCWSTR::from_raw(message.as_mut_ptr())];

        unsafe {
            WinEventLog::ReportEventW(
                self.handle,
                message_type.message_type,
                category,
                message_type.level,
                None,
                0,
                Some(pwstrs.as_slice()),
                None,
            )
        }?;

        Ok(())
    }
}

impl Drop for EventLog {
    fn drop(&mut self) {
        let result = unsafe { WinEventLog::DeregisterEventSource(self.handle) };
        if let Err(e) = result {
            println!("{e:?}");
        }
    }
}
