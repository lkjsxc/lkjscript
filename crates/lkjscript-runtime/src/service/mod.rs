use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File};
use std::path::Path;

mod templates;

const FILES: [&str; 6] = [
    "container-command.txt",
    "lkjscript-session.service",
    "lkjscriptd-windows-service.txt",
    "lkjscriptd.service",
    "org.lkjscript.daemon.plist",
    "org.lkjscript.session.plist",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceConfiguration {
    pub principal: u32,
    pub coordinator: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceError {
    InvalidCoordinator,
    Io(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoordinator => output.write_str("service coordinator must be nonzero"),
            Self::Io(message) => write!(output, "service definition I/O: {message}"),
        }
    }
}

impl std::error::Error for ServiceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceBundle {
    files: BTreeMap<&'static str, String>,
}

impl ServiceBundle {
    pub fn new(configuration: ServiceConfiguration) -> Result<Self, ServiceError> {
        if configuration.coordinator == 0 {
            return Err(ServiceError::InvalidCoordinator);
        }
        let files = BTreeMap::from([
            ("container-command.txt", templates::container(configuration)),
            ("lkjscript-session.service", templates::linux_session()),
            (
                "lkjscriptd-windows-service.txt",
                templates::windows(configuration),
            ),
            ("lkjscriptd.service", templates::linux_system(configuration)),
            (
                "org.lkjscript.daemon.plist",
                templates::macos_daemon(configuration),
            ),
            ("org.lkjscript.session.plist", templates::macos_agent()),
        ]);
        Ok(Self { files })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files
            .iter()
            .map(|(name, content)| (*name, content.as_str()))
    }

    pub fn write_to(&self, directory: &Path) -> Result<(), ServiceError> {
        fs::create_dir_all(directory).map_err(io_error)?;
        for (name, content) in &self.files {
            let path = directory.join(name);
            fs::write(&path, content).map_err(io_error)?;
            File::open(path)
                .and_then(|file| file.sync_all())
                .map_err(io_error)?;
        }
        File::open(directory)
            .and_then(|file| file.sync_all())
            .map_err(io_error)
    }

    pub fn remove_from(directory: &Path) -> Result<(), ServiceError> {
        for name in FILES {
            let path = directory.join(name);
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(io_error(error)),
            }
        }
        if directory.exists() {
            File::open(directory)
                .and_then(|file| file.sync_all())
                .map_err(io_error)?;
        }
        Ok(())
    }
}

fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_are_complete_deterministic_and_removable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let configuration = ServiceConfiguration {
            principal: 1000,
            coordinator: 9,
        };
        let first = ServiceBundle::new(configuration)?;
        assert_eq!(first, ServiceBundle::new(configuration)?);
        assert_eq!(first.iter().count(), FILES.len());
        let joined = first.iter().map(|(_, content)| content).collect::<String>();
        assert!(joined.contains("lkjscriptd --foreground"));
        assert!(joined.contains("lkjscript-session"));
        assert!(joined.contains("Session 0") || joined.contains("service never presents UI"));
        let directory =
            std::env::temp_dir().join(format!("lkjscript-services-{}", std::process::id()));
        let _ignored = fs::remove_dir_all(&directory);
        first.write_to(&directory)?;
        assert!(FILES.iter().all(|name| directory.join(name).is_file()));
        ServiceBundle::remove_from(&directory)?;
        assert!(FILES.iter().all(|name| !directory.join(name).exists()));
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
