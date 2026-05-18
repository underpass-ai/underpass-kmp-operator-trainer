use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use operator_shared_application::error::application_error::{ApplicationError, ApplicationResult};
use operator_shared_application::ports::trajectory_writer::TrajectoryWriter;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::errors::infra_error::InfraError;
use crate::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

/// `TrajectoryWriter` adapter that appends one JSONL row per trajectory.
/// Opens the file in append mode on the first write; subsequent writes
/// stream into the held `BufWriter`.
#[derive(Debug)]
pub struct JsonlTrajectoryWriter {
    path: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl JsonlTrajectoryWriter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            writer: None,
        }
    }

    fn ensure_open(&mut self) -> Result<&mut BufWriter<File>, InfraError> {
        if self.writer.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|e| InfraError::io("jsonl_trajectory_writer.open", e))?;
            self.writer = Some(BufWriter::new(file));
        }
        Ok(self
            .writer
            .as_mut()
            .expect("writer was just initialised above"))
    }

    fn write_internal(&mut self, trajectory: &TrainingTrajectory) -> Result<(), InfraError> {
        let dto = TrainingTrajectoryMapper::to_dto(trajectory)?;
        let line = serde_json::to_string(&dto)
            .map_err(|e| InfraError::serde("jsonl_trajectory_writer.serialize", &e))?;
        let writer = self.ensure_open()?;
        writer
            .write_all(line.as_bytes())
            .map_err(|e| InfraError::io("jsonl_trajectory_writer.write", e))?;
        writer
            .write_all(b"\n")
            .map_err(|e| InfraError::io("jsonl_trajectory_writer.newline", e))?;
        writer
            .flush()
            .map_err(|e| InfraError::io("jsonl_trajectory_writer.flush", e))?;
        Ok(())
    }
}

impl TrajectoryWriter for JsonlTrajectoryWriter {
    fn write(&mut self, trajectory: &TrainingTrajectory) -> ApplicationResult<()> {
        self.write_internal(trajectory)
            .map_err(ApplicationError::from)
    }
}
