mod commands;
mod local_api;
mod repository_open_target;
pub mod updater;

use std::sync::Mutex;
use tauri::Manager;

pub use commands::{
    local_api_bootstrap, open_external_url, open_repository_in_file_manager,
    open_repository_in_target, repository_open_targets, run_user_terminal_command,
    start_window_drag, toggle_window_maximized, LocalApiBootstrap,
};
pub use local_api::{PackagedLocalApi, PackagedLocalApiConfig, PackagedLocalApiError};

pub struct LocalApiServer(Mutex<Option<PackagedLocalApi>>);

impl LocalApiServer {
    pub fn shutdown(&self) {
        if let Some(api) = self
            .0
            .lock()
            .expect("local api lock should not be poisoned")
            .take()
        {
            let _ = api.shutdown();
        }
    }

    pub fn from_api_for_test(api: PackagedLocalApi) -> Self {
        Self(Mutex::new(Some(api)))
    }
}

impl Drop for LocalApiServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = packaged_local_api_config();
            let api = PackagedLocalApi::start(config)
                .expect("failed to bind DesktopLab packaged local API");
            app.manage(LocalApiServer(Mutex::new(Some(api))));
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                if let Some(server) = window.try_state::<LocalApiServer>() {
                    server.shutdown();
                }
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            local_api_bootstrap,
            open_repository_in_file_manager,
            repository_open_targets,
            open_repository_in_target,
            open_external_url,
            run_user_terminal_command,
            start_window_drag,
            toggle_window_maximized
        ])
        .build(tauri::generate_context!())
        .expect("failed to build DesktopLab desktop shell")
        .run(|app_handle, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                if let Some(server) = app_handle.try_state::<LocalApiServer>() {
                    server.shutdown();
                }
            }
        });
}

fn packaged_local_api_config() -> PackagedLocalApiConfig {
    packaged_local_api_config_from(
        packaged_user_home(),
        packaged_app_data_dir()
            .expect("DESKTOPLAB_APP_DATA_DIR must be an absolute path when it is configured"),
    )
}

fn packaged_local_api_config_from(
    home: Option<std::path::PathBuf>,
    app_data_dir: Option<std::path::PathBuf>,
) -> PackagedLocalApiConfig {
    let config = if let Some(app_data_dir) = app_data_dir {
        PackagedLocalApiConfig::for_app_data_dir(app_data_dir)
    } else if let Some(home) = home {
        PackagedLocalApiConfig::for_user_home(&home)
            .unwrap_or_else(|_| PackagedLocalApiConfig::random_loopback())
    } else {
        PackagedLocalApiConfig::random_loopback()
    };
    config.with_managed_ollama_shutdown(true)
}

fn packaged_user_home() -> Option<std::path::PathBuf> {
    packaged_user_home_from(
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
        cfg!(target_os = "windows"),
    )
}

fn packaged_app_data_dir() -> Result<Option<std::path::PathBuf>, &'static str> {
    packaged_app_data_dir_from(std::env::var_os("DESKTOPLAB_APP_DATA_DIR"))
}

fn packaged_app_data_dir_from(
    app_data_dir: Option<std::ffi::OsString>,
) -> Result<Option<std::path::PathBuf>, &'static str> {
    let Some(value) = app_data_dir.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let path = std::path::PathBuf::from(value);
    if !path.is_absolute() {
        return Err("app data path must be absolute");
    }
    Ok(Some(path))
}

fn packaged_user_home_from(
    home: Option<std::ffi::OsString>,
    user_profile: Option<std::ffi::OsString>,
    windows: bool,
) -> Option<std::path::PathBuf> {
    home.filter(|value| !value.is_empty())
        .or_else(|| {
            windows
                .then_some(user_profile)
                .flatten()
                .filter(|value| !value.is_empty())
        })
        .map(std::path::PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::{
        packaged_app_data_dir_from, packaged_local_api_config_from, packaged_user_home_from,
    };
    use crate::PackagedLocalApiConfig;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn packaged_home_prefers_home_and_supports_windows_user_profile() {
        assert_eq!(
            packaged_user_home_from(
                Some(OsString::from("/home/desktoplab")),
                Some(OsString::from(r"C:\Users\desktoplab")),
                true,
            ),
            Some(PathBuf::from("/home/desktoplab"))
        );
        assert_eq!(
            packaged_user_home_from(None, Some(OsString::from(r"C:\Users\desktoplab")), true,),
            Some(PathBuf::from(r"C:\Users\desktoplab"))
        );
        assert_eq!(
            packaged_user_home_from(None, Some(OsString::from("ignored")), false),
            None
        );
    }

    #[test]
    fn packaged_app_data_override_requires_an_absolute_path() {
        let absolute = if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\DesktopLab\isolated")
        } else {
            PathBuf::from("/tmp/desktoplab-isolated")
        };
        assert_eq!(
            packaged_app_data_dir_from(Some(absolute.clone().into_os_string())),
            Ok(Some(absolute))
        );
        assert_eq!(packaged_app_data_dir_from(Some(OsString::new())), Ok(None));
        assert!(packaged_app_data_dir_from(Some(OsString::from("relative/path"))).is_err());
    }

    #[test]
    fn packaged_app_data_override_is_independent_from_the_user_home() {
        let home = PathBuf::from("/Users/example");
        let app_data = PathBuf::from("/tmp/desktoplab-isolated");

        assert_eq!(
            packaged_local_api_config_from(Some(home), Some(app_data.clone())),
            PackagedLocalApiConfig::for_app_data_dir(app_data).with_managed_ollama_shutdown(true)
        );
    }

    #[test]
    fn packaged_config_falls_back_to_the_standard_user_home() {
        let home = PathBuf::from("/Users/example");

        assert_eq!(
            packaged_local_api_config_from(Some(home.clone()), None),
            PackagedLocalApiConfig::for_user_home(&home)
                .expect("user home config should build")
                .with_managed_ollama_shutdown(true)
        );
    }
}
