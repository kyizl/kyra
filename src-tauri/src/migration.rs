use std::fs;
use std::path::{Path, PathBuf};

fn is_ignorable_io_error(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::PermissionDenied
        || error.raw_os_error() == Some(32)
        || error.raw_os_error() == Some(33)
}

fn is_ephemeral_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_string_lossy().as_ref(),
            "Cache"
                | "Code Cache"
                | "DawnGraphiteCache"
                | "DawnWebGPUCache"
                | "GPUCache"
                | "Shared Dictionary"
                | "VideoDecodeStats"
                | "shared_proto_db"
        )
    })
}

fn is_ephemeral_root(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_string_lossy().as_ref(),
        "Cache"
            | "Code Cache"
            | "DawnGraphiteCache"
            | "DawnWebGPUCache"
            | "GPUCache"
            | "Shared Dictionary"
            | "VideoDecodeStats"
            | "shared_proto_db"
            | "EBWebView"
    )
}

fn copy_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if is_ephemeral_path(source) {
        return Ok(());
    }
    if source.is_file() {
        if !destination.exists() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            if let Err(error) = fs::copy(source, destination) {
                if !is_ignorable_io_error(&error) {
                    return Err(error.to_string());
                }
            }
        }
        return Ok(());
    }
    if !source.is_dir() {
        return Ok(());
    }
    let entries = match fs::read_dir(source) {
        Ok(entries) => entries,
        Err(error) if is_ignorable_io_error(&error) => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) if is_ignorable_io_error(&error) => continue,
            Err(error) => return Err(error.to_string()),
        };
        if is_ephemeral_root(&entry.file_name()) {
            continue;
        }
        let target = destination.join(entry.file_name());
        copy_missing(&entry.path(), &target)?;
    }
    Ok(())
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

pub fn migrate_legacy_data(app_data_dir: &Path, app_config_dir: &Path) -> Result<(), String> {
    let app_data = dirs::data_dir().ok_or_else(|| "user data directory unavailable".to_owned())?;
    let local_data =
        dirs::data_local_dir().ok_or_else(|| "local data directory unavailable".to_owned())?;
    let config = dirs::config_dir().ok_or_else(|| "config directory unavailable".to_owned())?;
    let config_local =
        dirs::config_local_dir().ok_or_else(|| "local config directory unavailable".to_owned())?;
    let candidates: [PathBuf; 10] = [
        app_data.join("kyra"),
        app_data.join("Kyra Overlay"),
        app_data.join("com.thebois.overlay"),
        local_data.join("kyra"),
        local_data.join("Kyra Overlay"),
        local_data.join("com.thebois.overlay"),
        config.join("kyra"),
        config.join("Kyra Overlay"),
        config_local.join("kyra"),
        config_local.join("Kyra Overlay"),
    ];
    for source in candidates {
        if paths_overlap(&source, app_data_dir) || paths_overlap(&source, app_config_dir) {
            continue;
        }
        if source.exists() {
            copy_missing(&source, app_data_dir)?;
            copy_missing(&source, app_config_dir)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{copy_missing, is_ephemeral_path, paths_overlap};
    use std::fs;

    #[test]
    fn copy_missing_preserves_existing_destination() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("settings.json"), b"old").unwrap();
        fs::write(destination.join("settings.json"), b"new").unwrap();
        copy_missing(&source, &destination).unwrap();
        assert_eq!(fs::read(destination.join("settings.json")).unwrap(), b"new");
    }

    #[test]
    fn paths_overlap_detects_nested_destinations() {
        let root = std::path::Path::new(r"C:\Users\test\AppData\Roaming\kyra");
        assert!(paths_overlap(root, &root.join("config")));
        assert!(paths_overlap(root, root));
        assert!(!paths_overlap(
            root,
            std::path::Path::new(r"C:\Users\test\AppData\Local\kyra")
        ));
    }

    #[test]
    fn ephemeral_browser_data_is_not_migrated() {
        assert!(is_ephemeral_path(std::path::Path::new(
            r"C:\Users\test\AppData\Roaming\kyra\Cache\data"
        )));
        assert!(!is_ephemeral_path(std::path::Path::new(
            r"C:\Users\test\AppData\Roaming\kyra\Local Storage\data"
        )));
    }
}
