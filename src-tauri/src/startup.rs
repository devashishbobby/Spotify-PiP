//! Per-user Windows sign-in startup. Only our named Run value is changed.
#[cfg(windows)]
mod windows {
    use std::{io, path::Path};
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const PREFERENCES_KEY: &str = r"Software\com.devas.spotifypip";
    const VALUE: &str = "SpotifyPiP";

    struct Registration {
        run: RegKey,
        preferences: RegKey,
    }

    impl Registration {
        fn open() -> io::Result<Self> {
            let user = RegKey::predef(HKEY_CURRENT_USER);
            Ok(Self {
                run: user.create_subkey(RUN_KEY)?.0,
                preferences: user.create_subkey(PREFERENCES_KEY)?.0,
            })
        }

        fn enabled(&self) -> io::Result<bool> {
            match self.run.get_value::<String, _>(VALUE) {
                Ok(command) => Ok(!command.is_empty()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            }
        }

        fn set(&self, enabled: bool, executable: &Path) -> io::Result<bool> {
            if enabled {
                // Quote the executable so paths containing spaces remain executable.
                let command = format!("\"{}\" --startup", executable.display());
                if command.encode_utf16().count() > 260 {
                    return Err(io::Error::other(
                        "The app path is too long for Windows startup. Move SpotifyPiP to a shorter path.",
                    ));
                }
                self.run.set_value(VALUE, &command)?;
            } else {
                match self.run.delete_value(VALUE) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            self.preferences.set_value("StartupConfigured", &1u32)?;
            self.enabled()
        }

        fn initialize(&self, executable: &Path) -> io::Result<()> {
            let configured = match self.preferences.get_value::<u32, _>("StartupConfigured") {
                Ok(value) => value != 0,
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(error) => return Err(error),
            };
            // Apply the default once. A disabled choice must survive every restart.
            // Refresh an enabled entry if the app was moved or installed elsewhere.
            if !configured || self.enabled()? {
                self.set(true, executable)?;
            }
            Ok(())
        }
    }

    pub fn initialize() -> Result<(), String> {
        let result = (|| Registration::open()?.initialize(&std::env::current_exe()?))();
        result.map_err(|error: io::Error| format!("Could not configure Windows startup: {error}"))
    }

    pub fn enabled() -> Result<bool, String> {
        Registration::open()
            .and_then(|entry| entry.enabled())
            .map_err(|error| format!("Could not read Windows startup: {error}"))
    }

    pub fn set(enabled: bool) -> Result<bool, String> {
        let result = (|| Registration::open()?.set(enabled, &std::env::current_exe()?))();
        result.map_err(|error: io::Error| format!("Could not change Windows startup: {error}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn startup_defaults_on_and_keeps_disabled_choice_across_relaunches() {
            // Exercise real registry operations in an isolated, non-startup key.
            let user = RegKey::predef(HKEY_CURRENT_USER);
            let test_path = format!(
                r"Software\SpotifyPiP-tests\{}-{}",
                std::process::id(),
                crate::media_session::state::utc_ms()
            );
            let root = user.create_subkey(&test_path).unwrap().0;
            let entry = Registration {
                run: root.create_subkey("Run").unwrap().0,
                preferences: root.create_subkey("Preferences").unwrap().0,
            };
            let executable = Path::new(r"C:\Music Apps\SpotifyPiP\spotify-pip.exe");
            entry.run.set_value("OtherApp", &"keep me").unwrap();
            entry.initialize(executable).unwrap();
            assert!(entry.enabled().unwrap());
            assert_eq!(
                entry.run.get_value::<String, _>(VALUE).unwrap(),
                r#""C:\Music Apps\SpotifyPiP\spotify-pip.exe" --startup"#
            );
            assert!(!entry.set(false, executable).unwrap());
            entry.initialize(executable).unwrap();
            assert!(!entry.enabled().unwrap());
            assert!(!entry.set(false, executable).unwrap());
            assert!(entry.set(true, executable).unwrap());
            let moved = Path::new(r"C:\New Location\spotify-pip.exe");
            entry.initialize(moved).unwrap();
            assert_eq!(
                entry.run.get_value::<String, _>(VALUE).unwrap(),
                r#""C:\New Location\spotify-pip.exe" --startup"#
            );
            assert_eq!(
                entry.run.get_value::<String, _>("OtherApp").unwrap(),
                "keep me"
            );
            drop(entry);
            drop(root);
            user.delete_subkey_all(&test_path).unwrap();
        }
    }
}

#[cfg(windows)]
pub use windows::{enabled, initialize, set};

#[cfg(not(windows))]
pub fn initialize() -> Result<(), String> {
    Ok(())
}
#[cfg(not(windows))]
pub fn enabled() -> Result<bool, String> {
    Err("Startup is available on Windows only.".into())
}
#[cfg(not(windows))]
pub fn set(_enabled: bool) -> Result<bool, String> {
    enabled()
}
