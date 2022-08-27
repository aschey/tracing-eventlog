#[cfg(windows)]
pub mod platform {
    use registry::{Data, Hive, RegKey, Security};
    use windows::Win32::System::EventLog::{
        EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE,
    };

    use crate::{error::RegistryError, eventmsgs};

    const REG_BASEKEY: &str = r"SYSTEM\CurrentControlSet\Services\EventLog\Application";

    pub fn register(name: &str) -> core::result::Result<(), RegistryError> {
        let current_exe = std::env::current_exe().map_err(RegistryError::SystemError)?;
        let exe_path = current_exe.to_str().ok_or(RegistryError::InvalidExePath)?;

        println!("exe path {exe_path}");
        let exe_path = &exe_path.replacen("\\\\?\\", "", 1);
        let key = Hive::LocalMachine
            .open(REG_BASEKEY, Security::Write)
            .map_err(map_key_error)?;
        let app_key = key.create(name, Security::Write).map_err(map_key_error)?;

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

    pub fn deregister(name: &str) -> core::result::Result<(), RegistryError> {
        let key = Hive::LocalMachine
            .open(REG_BASEKEY, Security::Read)
            .map_err(map_key_error)?;
        key.delete(name, true).map_err(map_key_error)?;
        Ok(())
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
}

#[cfg(not(windows))]
pub mod platform {
    pub fn register(name: &str) {}

    pub fn deregister(name: &str) {}
}
