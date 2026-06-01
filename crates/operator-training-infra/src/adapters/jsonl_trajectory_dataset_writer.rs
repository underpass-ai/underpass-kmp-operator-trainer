//! Filesystem `DatasetWriter` that emits one JSON Lines record per
//! `TrainingTrajectory` in the rich `TrainingTrajectoryDto` shape:
//! `{id, step_id, about, mode, task_family, goal, allowed_tools, visible_state,
//! target_action}`.
//!
//! This is the schema the Python SFT-prep pipeline consumes
//! (`prepare_operator_sft_dataset.py --trajectories`) to build the
//! `{system,user,assistant}` chat dataset: its `to_sft_row` reads `about`,
//! `goal`, `mode`, `task_family`, `allowed_tools`, `visible_state`, and
//! `target_action` off each item. The thin `JsonlSftDatasetWriter`
//! (`{prompt,completion}`) drops about/goal, so it cannot feed that pipeline —
//! this writer carries them so the user message conditions on the goal/about.
//!
//! Same durability contract as the SFT writer: always newline-terminated, the
//! bytes are SHA-256-hashed in flight, and `sync_all` runs after the last write.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;
use operator_training_application::errors::dataset_write_error::DatasetWriteError;
use operator_training_application::ports::dataset_write_outcome::DatasetWriteOutcome;
use operator_training_application::ports::dataset_writer::DatasetWriter;
use operator_training_domain::provenance::content_hash::ContentHash;
use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
use operator_training_domain::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;
use sha2::{Digest, Sha256};

const ADAPTER: &str = "jsonl_trajectory_dataset_writer";

#[derive(Debug, Clone)]
pub struct JsonlTrajectoryDatasetWriter {
    target_path: PathBuf,
}

impl JsonlTrajectoryDatasetWriter {
    pub fn new(target_path: impl Into<PathBuf>) -> Self {
        Self {
            target_path: target_path.into(),
        }
    }

    pub fn target_path(&self) -> &std::path::Path {
        &self.target_path
    }
}

impl DatasetWriter for JsonlTrajectoryDatasetWriter {
    fn write(
        &self,
        trajectories: &[TrainingTrajectory],
    ) -> Result<DatasetWriteOutcome, DatasetWriteError> {
        let file =
            File::create(&self.target_path).map_err(|err| DatasetWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("create {}: {err}", self.target_path.display()),
            })?;
        let mut writer = BufWriter::new(file);
        let mut hasher = Sha256::new();
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();

        for trajectory in trajectories {
            let line = build_line(trajectory)?;
            let bytes = line.into_bytes();
            hasher.update(&bytes);
            writer
                .write_all(&bytes)
                .map_err(|err| DatasetWriteError::WriteFailure {
                    adapter: ADAPTER,
                    message: format!("write line: {err}"),
                })?;
            hasher.update(b"\n");
            writer
                .write_all(b"\n")
                .map_err(|err| DatasetWriteError::WriteFailure {
                    adapter: ADAPTER,
                    message: format!("write newline: {err}"),
                })?;

            *counts
                .entry(trajectory.task_family().as_str().to_string())
                .or_insert(0) += 1;
        }

        writer
            .flush()
            .map_err(|err| DatasetWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("flush: {err}"),
            })?;
        let file = writer
            .into_inner()
            .map_err(|err| DatasetWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("unwrap BufWriter: {}", err.error()),
            })?;
        file.sync_all()
            .map_err(|err| DatasetWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("sync_all: {err}"),
            })?;

        let digest = hasher.finalize();
        let content_hash = ContentHash::parse(format!("sha256:{digest:x}")).map_err(|err| {
            DatasetWriteError::DerivedValueFailure {
                adapter: ADAPTER,
                message: format!("content_hash: {err}"),
            }
        })?;

        let trajectory_count = PositiveCount::parse(trajectories.len(), "trajectory_count")
            .map_err(|err| DatasetWriteError::DerivedValueFailure {
                adapter: ADAPTER,
                message: format!("trajectory_count: {err}"),
            })?;

        let distribution = build_distribution(counts)?;

        Ok(DatasetWriteOutcome::new(
            content_hash,
            trajectory_count,
            distribution,
        ))
    }
}

fn build_line(trajectory: &TrainingTrajectory) -> Result<String, DatasetWriteError> {
    let dto = TrainingTrajectoryMapper::to_dto(trajectory).map_err(|err| {
        DatasetWriteError::DerivedValueFailure {
            adapter: ADAPTER,
            message: format!("map trajectory to dto: {err}"),
        }
    })?;
    serde_json::to_string(&dto).map_err(|err| DatasetWriteError::DerivedValueFailure {
        adapter: ADAPTER,
        message: format!("serialize trajectory: {err}"),
    })
}

fn build_distribution(
    counts: BTreeMap<String, usize>,
) -> Result<TaskFamilyDistribution, DatasetWriteError> {
    let mut entries = Vec::with_capacity(counts.len());
    for (family_str, count) in counts {
        let family = TaskFamily::parse(family_str.clone()).map_err(|err| {
            DatasetWriteError::DerivedValueFailure {
                adapter: ADAPTER,
                message: format!("task_family {family_str}: {err}"),
            }
        })?;
        let positive = PositiveCount::parse(count, "task_family_count").map_err(|err| {
            DatasetWriteError::DerivedValueFailure {
                adapter: ADAPTER,
                message: format!("task_family_count for {family_str}: {err}"),
            }
        })?;
        entries.push(TaskFamilyDistributionEntry::new(family, positive));
    }
    TaskFamilyDistribution::new(entries).map_err(|err| DatasetWriteError::DerivedValueFailure {
        adapter: ADAPTER,
        message: format!("distribution: {err}"),
    })
}
