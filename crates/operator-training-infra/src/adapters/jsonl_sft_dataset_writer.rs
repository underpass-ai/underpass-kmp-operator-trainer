//! Filesystem `DatasetWriter` that emits one JSON Lines record per
//! `TrainingTrajectory`, shaped `{"prompt": <text>, "completion": <json>}`
//! per ADR 0012 §3.
//!
//! - `prompt` is `serde_json::to_string(&VisibleStateDto)` — a
//!   stable, lossless JSON serialisation of the visible state the
//!   operator policy sees at decision time. Future PRs may swap this
//!   for a natural-language renderer behind a `PromptRenderer` port;
//!   the SFT line shape stays the same.
//! - `completion` is `serde_json::to_string(&OperatorActionDto)` —
//!   the JSON encoding of the target action.
//!
//! Format contract:
//!
//! - Output is **always newline-terminated**: even the last record is
//!   followed by a `\n`. Consumers that split on lines see N records
//!   for N trajectories with no trailing-empty-line surprise.
//! - Bytes written to disk are SHA-256-hashed in flight and the
//!   per-task-family distribution is built from the same trajectories,
//!   so the returned `DatasetWriteOutcome` is self-consistent.
//! - After the last write the writer calls `flush` *and* `sync_all`
//!   so the bytes are durable on the underlying storage, not just
//!   buffered in the OS page cache. Callers can rely on the file
//!   surviving a power loss the moment `write` returns `Ok`.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_shared_infra::mappers::visible_state_mapper::VisibleStateMapper;
use operator_training_application::errors::dataset_write_error::DatasetWriteError;
use operator_training_application::ports::dataset_write_outcome::DatasetWriteOutcome;
use operator_training_application::ports::dataset_writer::DatasetWriter;
use operator_training_domain::provenance::content_hash::ContentHash;
use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
use operator_training_domain::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;
use sha2::{Digest, Sha256};

use crate::dto::jsonl_sft_example_dto::JsonlSftExampleDto;

const ADAPTER: &str = "jsonl_sft_dataset_writer";

#[derive(Debug, Clone)]
pub struct JsonlSftDatasetWriter {
    target_path: PathBuf,
}

impl JsonlSftDatasetWriter {
    pub fn new(target_path: impl Into<PathBuf>) -> Self {
        Self {
            target_path: target_path.into(),
        }
    }

    pub fn target_path(&self) -> &std::path::Path {
        &self.target_path
    }
}

impl DatasetWriter for JsonlSftDatasetWriter {
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
            let example = build_example(trajectory)?;
            let line = serde_json::to_string(&example).map_err(|err| {
                DatasetWriteError::DerivedValueFailure {
                    adapter: ADAPTER,
                    message: format!("serialize example: {err}"),
                }
            })?;
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
        // `flush` only drains the user-space buffer into the OS page
        // cache; `sync_all` is what guarantees the bytes are on disk.
        // We extract the inner `File` so we can call `sync_all` on it
        // directly — `BufWriter` has no equivalent.
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

fn build_example(trajectory: &TrainingTrajectory) -> Result<JsonlSftExampleDto, DatasetWriteError> {
    let visible_dto = VisibleStateMapper::to_dto(trajectory.visible_state());
    let prompt = serde_json::to_string(&visible_dto).map_err(|err| {
        DatasetWriteError::DerivedValueFailure {
            adapter: ADAPTER,
            message: format!("render prompt: {err}"),
        }
    })?;
    let action_dto = OperatorActionMapper::to_dto(trajectory.target_action()).map_err(|err| {
        DatasetWriteError::DerivedValueFailure {
            adapter: ADAPTER,
            message: format!("map target_action: {err}"),
        }
    })?;
    let completion = serde_json::to_string(&action_dto).map_err(|err| {
        DatasetWriteError::DerivedValueFailure {
            adapter: ADAPTER,
            message: format!("render completion: {err}"),
        }
    })?;
    Ok(JsonlSftExampleDto { prompt, completion })
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
