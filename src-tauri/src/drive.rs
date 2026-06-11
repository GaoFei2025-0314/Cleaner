use sysinfo::Disks;

use crate::errors::CleanerError;
use crate::models::DriveSummary;

pub fn c_drive_summary() -> Result<DriveSummary, CleanerError> {
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let mount = disk
            .mount_point()
            .to_string_lossy()
            .replace('/', "\\")
            .to_uppercase();
        if mount == "C:\\" || mount == "C:" {
            return Ok(DriveSummary {
                drive: "C:".to_string(),
                total_bytes: disk.total_space(),
                free_bytes: disk.available_space(),
            });
        }
    }
    Err(CleanerError::PathResolution("C drive was not found".to_string()))
}
