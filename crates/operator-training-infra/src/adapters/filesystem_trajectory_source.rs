//! `TrajectorySource` adapter that reads one `TrainingTrajectoryDto`
//! per JSONL line from a path on disk and maps each row into a
//! `TrainingTrajectory` via `operator-shared-infra`'s
//! `TrainingTrajectoryMapper`. Empty / whitespace-only lines are
//! skipped silently; the first row with bad shape surfaces an
//! `InvalidTrajectory` error with its line index.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;
use operator_training_application::errors::trajectory_source_error::TrajectorySourceError;
use operator_training_application::ports::trajectory_source::TrajectorySource;

const ADAPTER: &str = "filesystem_trajectory_source";

#[derive(Debug, Clone)]
pub struct FilesystemTrajectorySource {
    source_path: PathBuf,
}

impl FilesystemTrajectorySource {
    pub fn new(source_path: impl Into<PathBuf>) -> Self {
        Self {
            source_path: source_path.into(),
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }
}

impl TrajectorySource for FilesystemTrajectorySource {
    fn fetch_all(&self) -> Result<Vec<TrainingTrajectory>, TrajectorySourceError> {
        let file =
            File::open(&self.source_path).map_err(|err| TrajectorySourceError::Unavailable {
                adapter: ADAPTER,
                message: format!("open {}: {err}", self.source_path.display()),
            })?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|err| TrajectorySourceError::Unavailable {
                adapter: ADAPTER,
                message: format!("read line {}: {err}", index + 1),
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: TrainingTrajectoryDto = serde_json::from_str(&line).map_err(|err| {
                TrajectorySourceError::InvalidTrajectory {
                    adapter: ADAPTER,
                    index,
                    message: format!("parse JSONL: {err}"),
                }
            })?;
            let trajectory = TrainingTrajectoryMapper::to_domain(&dto).map_err(|err| {
                TrajectorySourceError::InvalidTrajectory {
                    adapter: ADAPTER,
                    index,
                    message: format!("map to domain: {err}"),
                }
            })?;
            out.push(trajectory);
        }
        Ok(out)
    }
}
