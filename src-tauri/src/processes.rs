use std::path::Path;

use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessReference {
    pub process_name: String,
}

pub fn find_process_references(candidate_path: &Path) -> Vec<ProcessReference> {
    let candidate = normalize(candidate_path);
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut refs = Vec::new();
    for process in system.processes().values() {
        let exe_matches = process
            .exe()
            .map(|path| normalize(path).starts_with(&candidate))
            .unwrap_or(false);
        let cmd_matches = process
            .cmd()
            .iter()
            .any(|part| part.to_string_lossy().to_lowercase().contains(&candidate));

        if exe_matches || cmd_matches {
            refs.push(ProcessReference {
                process_name: process.name().to_string_lossy().to_string(),
            });
        }
    }

    refs.sort_by(|left, right| left.process_name.cmp(&right.process_name));
    refs.dedup();
    refs
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}
