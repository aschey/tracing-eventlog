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
    use registry::{Data, Hive, RegKey, Security};
    use windows::Win32::System::EventLog::{
        EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE,
    };

    const REG_BASEKEY: &str = r"SYSTEM\CurrentControlSet\Services\EventLog";

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
            app_key: RegKey,
            exe_path: &str,
        ) -> core::result::Result<(), RegistryError> {
            let exe_path_value = Data::String(
                exe_path
                    .try_into()
                    .map_err(RegistryError::StrConvertError)?,
            );
            set_registry_value(&app_key, "EventMessageFile", &exe_path_value)?;

            set_registry_value(&app_key, "CategoryMessageFile", &exe_path_value)?;

            set_registry_value(
                &app_key,
                "CategoryCount",
                &Data::U32(eventmsgs::CATEGORY_COUNT),
            )?;

            let supported_types =
                EVENTLOG_ERROR_TYPE.0 | EVENTLOG_WARNING_TYPE.0 | EVENTLOG_INFORMATION_TYPE.0;
            set_registry_value(
                &app_key,
                "TypesSupported",
                &Data::U32(supported_types as u32),
            )?;

            Ok(())
        }
    }

    fn set_registry_value(
        key: &RegKey,
        name: &str,
        data: &Data,
    ) -> core::result::Result<(), RegistryError> {
        match key.set_value(name, data) {
            Ok(()) => Ok(()),
            Err(e) if matches!(e, registry::value::Error::PermissionDenied(_, _)) => {
                Err(RegistryError::PermissionDenied(registry::Error::Value(e)))
            }
            Err(e) => Err(RegistryError::ValueError(e)),
        }
    }

    fn map_key_error(result: registry::key::Error) -> RegistryError {
        match result {
            registry::key::Error::PermissionDenied(_, _) => {
                RegistryError::PermissionDenied(registry::Error::Key(result))
            }
            _ => RegistryError::KeyError(result),
        }
    }

    fn open_app_key(security: Security) -> core::result::Result<RegKey, RegistryError> {
        let key = Hive::LocalMachine
            .open(REG_BASEKEY, security)
            .map_err(map_key_error)?;
        let app_key = key.open("Application", security).map_err(map_key_error)?;
        Ok(app_key)
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
                    let app_key_read = open_app_key(Security::Read)?;
                    if app_key_read.open(&self.name, Security::Read).is_ok() {
                        return Ok(());
                    }

                    let app_key = open_app_key(Security::Write)?;
                    let name_key = app_key
                        .create(&self.name, Security::Write)
                        .map_err(map_key_error)?;
                    self.add_keys(name_key, exe_path)?;
                }
                SourceType::Custom(sources) => {
                    let base_key_read = Hive::LocalMachine
                        .open(REG_BASEKEY, Security::Read)
                        .map_err(map_key_error)?;
                    for source in sources {
                        if let Ok(custom_key) = base_key_read.open(&self.name, Security::Read) {
                            if custom_key.open(&self.name, Security::Read).is_ok()
                                && custom_key.open(source, Security::Read).is_ok()
                                && custom_key.value("AutoBackupLogFiles").is_ok()
                                && custom_key.value("MaxSize").is_ok()
                            {
                                continue;
                            }
                        }

                        let base_key = Hive::LocalMachine
                            .open(REG_BASEKEY, Security::Read)
                            .map_err(map_key_error)?;
                        let custom_key = base_key
                            .create(&self.name, Security::Write)
                            .map_err(map_key_error)?;
                        set_registry_value(&custom_key, "AutoBackupLogFiles", &Data::U32(0))?;
                        set_registry_value(&custom_key, "MaxSize", &Data::U32(0x00080000))?;
                        let name_key = custom_key
                            .create(&self.name, Security::Write)
                            .map_err(map_key_error)?;
                        self.add_keys(name_key, exe_path)?;
                        let source_key = custom_key
                            .create(source, Security::Write)
                            .map_err(map_key_error)?;
                        self.add_keys(source_key, exe_path)?;
                    }
                }
            };

            Ok(())
        }

        fn deregister(self) -> core::result::Result<(), RegistryError> {
            match self.source {
                SourceType::Application => {
                    let app_key = open_app_key(Security::Write)?;
                    app_key.delete(self.name, true).map_err(map_key_error)?;
                }
                SourceType::Custom(_) => {
                    let base_key = Hive::LocalMachine
                        .open(REG_BASEKEY, Security::Read)
                        .map_err(map_key_error)?;
                    base_key.delete(self.name, true).map_err(map_key_error)?;
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

        fn deregister(name: impl AsRef<str>) -> core::result::Result<(), RegistryError> {
            Ok(())
        }
    }
}
