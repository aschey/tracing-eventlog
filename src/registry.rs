use crate::error::RegistryError;

pub trait EventLogRegistry {
    fn application(name: impl Into<String>) -> Self;
    fn custom<'a>(name: impl Into<String>, sources: impl Into<Option<Vec<&'a str>>>) -> Self;
    fn register(&self) -> core::result::Result<(), RegistryError>;
    fn deregister(self) -> core::result::Result<(), RegistryError>;
}

#[cfg(windows)]
pub mod platform {
    use super::EventLogRegistry;
    use crate::{error::RegistryError, eventmsgs};
    use windows::Win32::{
        Foundation::ERROR_ACCESS_DENIED,
        System::EventLog::{EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE},
    };
    use windows_registry::{Key, LOCAL_MACHINE, Value};

    const REG_BASEKEY: &str = r"SYSTEM\CurrentControlSet\Services\EventLog";

    const APPLICATION: &str = "Application";

    enum SourceType {
        Application,
        Custom(Vec<String>),
    }

    pub struct LogSource {
        source: SourceType,
        name: String,
    }

    impl LogSource {
        fn add_keys(
            &self,
            app_key: Key,
            exe_path: &str,
        ) -> core::result::Result<(), RegistryError> {
            set_registry_value(&app_key, "EventMessageFile", &exe_path.into())?;

            set_registry_value(&app_key, "CategoryMessageFile", &exe_path.into())?;

            set_registry_value(&app_key, "CategoryCount", &eventmsgs::CATEGORY_COUNT.into())?;

            let supported_types =
                EVENTLOG_ERROR_TYPE.0 | EVENTLOG_WARNING_TYPE.0 | EVENTLOG_INFORMATION_TYPE.0;
            set_registry_value(&app_key, "TypesSupported", &(supported_types as u32).into())?;

            Ok(())
        }
    }

    fn set_registry_value(
        key: &Key,
        name: &str,
        data: &Value,
    ) -> core::result::Result<(), RegistryError> {
        key.set_value(name, data).map_err(|e| {
            if e.code() == ERROR_ACCESS_DENIED.into() {
                RegistryError::PermissionDenied(e)
            } else {
                RegistryError::ValueError(e)
            }
        })
    }

    fn map_key_error(result: ::windows::core::Error) -> RegistryError {
        if result.code() == ERROR_ACCESS_DENIED.into() {
            RegistryError::PermissionDenied(result)
        } else {
            RegistryError::KeyError(result)
        }
    }

    fn read_log_key() -> core::result::Result<Key, RegistryError> {
        LOCAL_MACHINE
            .options()
            .read()
            .open(REG_BASEKEY)
            .map_err(map_key_error)
    }

    fn write_log_key() -> core::result::Result<Key, RegistryError> {
        LOCAL_MACHINE
            .options()
            .read()
            .write()
            .create()
            .open(REG_BASEKEY)
            .map_err(map_key_error)
    }

    impl EventLogRegistry for LogSource {
        fn application(name: impl Into<String>) -> Self {
            Self {
                source: SourceType::Application,
                name: name.into(),
            }
        }

        fn custom<'a>(name: impl Into<String>, sources: impl Into<Option<Vec<&'a str>>>) -> Self {
            let name = name.into();
            let sources = sources.into().unwrap_or_default();
            let mut sources: Vec<String> = sources.into_iter().map(|s| s.into()).collect();
            sources.push(name.clone());
            Self {
                source: SourceType::Custom(sources),
                name,
            }
        }

        fn register(&self) -> core::result::Result<(), RegistryError> {
            let current_exe = std::env::current_exe().map_err(RegistryError::SystemError)?;
            let exe_path = current_exe.to_str().ok_or(RegistryError::InvalidExePath)?;

            let exe_path = &exe_path.replacen("\\\\?\\", "", 1);

            match &self.source {
                SourceType::Application => {
                    let app_key_read = read_log_key()?.open(APPLICATION).map_err(map_key_error)?;
                    if app_key_read.open(&self.name).is_ok() {
                        return Ok(());
                    }

                    let app_key = write_log_key()?.open(APPLICATION).map_err(map_key_error)?;
                    let name_key = app_key.create(&self.name).map_err(map_key_error)?;
                    self.add_keys(name_key, exe_path)?;
                }
                SourceType::Custom(sources) => {
                    let base_key_read = read_log_key()?;
                    for source in sources {
                        if let Ok(custom_key) = base_key_read.open(&self.name) {
                            if custom_key.open(&self.name).is_ok()
                                && custom_key.open(source).is_ok()
                                && custom_key.get_value("AutoBackupLogFiles").is_ok()
                                && custom_key.get_value("MaxSize").is_ok()
                            {
                                continue;
                            }
                        }

                        let base_key = read_log_key()?;
                        let custom_key = base_key.create(&self.name).map_err(map_key_error)?;
                        set_registry_value(&custom_key, "AutoBackupLogFiles", &0u32.into())?;
                        set_registry_value(&custom_key, "MaxSize", &0x00080000u32.into())?;
                        let name_key = custom_key.create(&self.name).map_err(map_key_error)?;
                        self.add_keys(name_key, exe_path)?;
                        let source_key = custom_key.create(source).map_err(map_key_error)?;
                        self.add_keys(source_key, exe_path)?;
                    }
                }
            };

            Ok(())
        }

        fn deregister(self) -> core::result::Result<(), RegistryError> {
            match self.source {
                SourceType::Application => {
                    let app_key = write_log_key()?.open(APPLICATION).map_err(map_key_error)?;
                    app_key.remove_tree(self.name).map_err(map_key_error)?;
                }
                SourceType::Custom(_) => {
                    let base_key = write_log_key()?;
                    base_key.remove_tree(self.name).map_err(map_key_error)?;
                }
            }
            Ok(())
        }
    }
}

#[cfg(not(windows))]
pub mod platform {
    use super::EventLogRegistry;
    use crate::error::RegistryError;
    pub struct LogSource;

    impl EventLogRegistry for LogSource {
        fn application(name: impl Into<String>) -> Self {
            Self {}
        }

        fn custom<'a>(name: impl Into<String>, sources: impl Into<Option<Vec<&'a str>>>) -> Self {
            Self {}
        }

        fn register(&self) -> core::result::Result<(), RegistryError> {
            Ok(())
        }

        fn deregister(self) -> core::result::Result<(), RegistryError> {
            Ok(())
        }
    }
}
