use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use operator_shared_application::error::application_error::{ApplicationError, ApplicationResult};
use operator_shared_application::ports::trajectory_reader::TrajectoryReader;
use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::errors::infra_error::InfraError;
use crate::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

/// `TrajectoryReader` adapter backed by a JSONL file on the local file
/// system. Lines that fail to parse abort the read (no silent skipping —
/// fail fast).
#[derive(Debug)]
pub struct JsonlTrajectoryReader {
    path: PathBuf,
}

impl JsonlTrajectoryReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    fn read_internal(&self) -> Result<Vec<TrainingTrajectory>, InfraError> {
        let file = File::open(&self.path)
            .map_err(|e| InfraError::io("jsonl_trajectory_reader.open", e))?;
        let reader = BufReader::new(file);
        let mut trajectories = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| InfraError::io("jsonl_trajectory_reader.line", e))?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: TrainingTrajectoryDto = serde_json::from_str(&line)
                .map_err(|e| InfraError::serde("jsonl_trajectory_reader.parse", &e))?;
            let trajectory = TrainingTrajectoryMapper::to_domain(&dto)?;
            trajectories.push(trajectory);
        }
        Ok(trajectories)
    }
}

impl TrajectoryReader for JsonlTrajectoryReader {
    fn read_all(&self) -> ApplicationResult<Vec<TrainingTrajectory>> {
        self.read_internal().map_err(ApplicationError::from)
    }
}
