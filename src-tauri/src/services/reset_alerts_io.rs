use serde::Serialize;
use std::{io::Write, path::Path, time::Duration};

pub fn save(path: &Path, value: &impl Serialize) -> Result<(), ()> {
    use std::os::unix::fs::PermissionsExt;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or(())?).map_err(|_| ())?;
    file.as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(|_| ())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| ())?;
    file.write_all(&bytes).map_err(|_| ())?;
    file.as_file().sync_all().map_err(|_| ())?;
    file.persist(path).map_err(|_| ())?;
    Ok(())
}

pub fn finish_cycle(path: &Path, value: &impl Serialize, elapsed: Duration) -> Duration {
    if save(path, value).is_err() {
        log::warn!("[ResetAlerts] Could not update diagnostic status; polling continues");
    }
    Duration::from_secs(300).saturating_sub(elapsed)
}
