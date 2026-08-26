use std::path::{Component, Path, PathBuf};
use typaxis_core::{HostPath, PortablePath, PortablePathError};

use crate::MachineInputErrorKind;

pub(crate) struct ResolvedPackageLocation {
    pub(crate) root: HostPath,
    pub(crate) uri: PortablePath,
}

pub(crate) fn resolve_package_location(
    package: &HostPath,
    explicit_root: Option<&HostPath>,
) -> Result<ResolvedPackageLocation, MachineInputErrorKind> {
    match explicit_root {
        Some(root) => {
            let current_directory = std::env::current_dir()
                .map_err(|_| MachineInputErrorKind::CurrentDirectoryUnavailable)?;
            resolve_explicit(package, root, &current_directory)
        }
        None => resolve_default(package),
    }
}

fn resolve_explicit(
    package: &HostPath,
    root: &HostPath,
    current_directory: &Path,
) -> Result<ResolvedPackageLocation, MachineInputErrorKind> {
    let package = absolute_preserving_components(package.as_path(), current_directory);
    let root = lexical_absolute(root.as_path(), current_directory);
    let relative = package
        .strip_prefix(&root)
        .map_err(|_| MachineInputErrorKind::PackageOutsideRoot)?;
    let uri = portable_relative_path(relative)?;
    let root = HostPath::new(root).map_err(|_| MachineInputErrorKind::InvalidPackagePath)?;
    Ok(ResolvedPackageLocation { root, uri })
}

fn resolve_default(package: &HostPath) -> Result<ResolvedPackageLocation, MachineInputErrorKind> {
    let package = package.as_path();
    let parent = package
        .parent()
        .ok_or(MachineInputErrorKind::InvalidPackagePath)?;
    let leaf = package
        .file_name()
        .ok_or(MachineInputErrorKind::InvalidPackagePath)?;
    let leaf = leaf
        .to_str()
        .ok_or(MachineInputErrorKind::NonPortablePackageUri)?;
    let uri = PortablePath::new(leaf.to_owned()).map_err(map_package_uri_error)?;
    let root = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        let current_directory = std::env::current_dir()
            .map_err(|_| MachineInputErrorKind::CurrentDirectoryUnavailable)?;
        current_directory.join(parent)
    };
    let root = HostPath::new(root).map_err(|_| MachineInputErrorKind::InvalidPackagePath)?;
    Ok(ResolvedPackageLocation { root, uri })
}

fn portable_relative_path(path: &Path) -> Result<PortablePath, MachineInputErrorKind> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => components.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(MachineInputErrorKind::PackageOutsideRoot);
                }
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(MachineInputErrorKind::InvalidPackagePath);
            }
        }
    }
    let mut value = String::new();
    for component in components {
        let component = component
            .to_str()
            .ok_or(MachineInputErrorKind::NonPortablePackageUri)?;
        if !value.is_empty() {
            value.push('/');
        }
        value.push_str(component);
    }
    PortablePath::new(value).map_err(map_package_uri_error)
}

fn absolute_preserving_components(path: &Path, current_directory: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_directory.join(path)
    }
}

fn map_package_uri_error(error: PortablePathError) -> MachineInputErrorKind {
    MachineInputErrorKind::InvalidPackageUri(error)
}

fn lexical_absolute(path: &Path, current_directory: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_directory.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_absolute_collapses_dot_and_parent_components() {
        let current = Path::new("/work/current");
        assert_eq!(
            lexical_absolute(Path::new("job/../package.json"), current),
            Path::new("/work/current/package.json")
        );
        assert_eq!(
            lexical_absolute(Path::new("/work/./job/package.json"), current),
            Path::new("/work/job/package.json")
        );
    }

    #[test]
    fn portable_relative_path_collapses_only_in_root_parent_components() {
        assert_eq!(
            portable_relative_path(Path::new("nested/../package.json"))
                .unwrap()
                .as_str(),
            "package.json"
        );
        assert!(matches!(
            portable_relative_path(Path::new("../root/package.json")),
            Err(MachineInputErrorKind::PackageOutsideRoot)
        ));
    }
}
