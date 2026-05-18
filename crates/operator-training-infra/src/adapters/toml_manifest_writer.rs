//! Filesystem `ManifestWriter` that serialises a `TrainingManifest`
//! to a TOML file per ADR 0012 §4. The mapping from the domain
//! manifest to the TOML DTO is deterministic; the writer never
//! re-orders gates or rewrites readiness reasons.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use operator_training_application::errors::manifest_write_error::ManifestWriteError;
use operator_training_application::ports::manifest_writer::ManifestWriter;
use operator_training_domain::manifest::training_manifest::TrainingManifest;
use operator_training_domain::readiness::readiness_check::ReadinessCheck;
use operator_training_domain::readiness::readiness_gate::ReadinessGate;
use operator_training_domain::readiness::readiness_outcome::ReadinessOutcome;
use operator_training_domain::readiness::readiness_report::ReadinessReport;

use crate::dto::manifest_dataset_dto::ManifestDatasetDto;
use crate::dto::manifest_dto::ManifestDto;
use crate::dto::manifest_readiness_dto::ManifestReadinessDto;
use crate::dto::manifest_readiness_gate_dto::ManifestReadinessGateDto;
use crate::dto::manifest_run_dto::ManifestRunDto;
use crate::dto::manifest_trainer_target_dto::ManifestTrainerTargetDto;

const ADAPTER: &str = "toml_manifest_writer";

#[derive(Debug, Clone)]
pub struct TomlManifestWriter {
    target_path: PathBuf,
}

impl TomlManifestWriter {
    pub fn new(target_path: impl Into<PathBuf>) -> Self {
        Self {
            target_path: target_path.into(),
        }
    }

    pub fn target_path(&self) -> &std::path::Path {
        &self.target_path
    }
}

impl ManifestWriter for TomlManifestWriter {
    fn write(&self, manifest: &TrainingManifest) -> Result<(), ManifestWriteError> {
        let dto = build_manifest_dto(manifest);
        let text = toml::to_string_pretty(&dto).map_err(|err| {
            ManifestWriteError::SerialisationFailure {
                adapter: ADAPTER,
                message: format!("toml serialisation failed: {err}"),
            }
        })?;
        let mut file =
            File::create(&self.target_path).map_err(|err| ManifestWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("create {}: {err}", self.target_path.display()),
            })?;
        file.write_all(text.as_bytes())
            .map_err(|err| ManifestWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("write: {err}"),
            })?;
        file.flush()
            .map_err(|err| ManifestWriteError::WriteFailure {
                adapter: ADAPTER,
                message: format!("flush: {err}"),
            })?;
        Ok(())
    }
}

fn build_manifest_dto(manifest: &TrainingManifest) -> ManifestDto {
    ManifestDto {
        run: ManifestRunDto {
            id: manifest.run_id().as_str().to_string(),
        },
        dataset: build_dataset_dto(manifest),
        readiness: build_readiness_dto(manifest.readiness()),
        trainer_target: ManifestTrainerTargetDto {
            command: manifest.trainer_target().command().as_str().to_string(),
            base_model: manifest.trainer_target().base_model().as_str().to_string(),
            output_directory: manifest
                .trainer_target()
                .output_directory()
                .as_str()
                .to_string(),
        },
    }
}

fn build_dataset_dto(manifest: &TrainingManifest) -> ManifestDatasetDto {
    let mut task_family_distribution = BTreeMap::new();
    for entry in manifest.dataset().distribution().entries() {
        task_family_distribution.insert(
            entry.family().as_str().to_string(),
            entry.count().as_usize(),
        );
    }
    ManifestDatasetDto {
        source: manifest.dataset().source().as_str().to_string(),
        content_hash: manifest.dataset().content_hash().as_str().to_string(),
        trajectory_count: manifest.dataset().trajectory_count().as_usize(),
        task_family_distribution,
    }
}

fn build_readiness_dto(report: &ReadinessReport) -> ManifestReadinessDto {
    let overall = if report.is_ready() {
        "ready"
    } else {
        "not_ready"
    };
    let gates = report.checks().iter().map(build_gate_dto).collect();
    ManifestReadinessDto {
        overall,
        gate: gates,
    }
}

fn build_gate_dto(check: &ReadinessCheck) -> ManifestReadinessGateDto {
    let (kind, target) = describe_gate(check.gate());
    let (status, reason) = match check.outcome() {
        ReadinessOutcome::Passed => ("passed", None),
        ReadinessOutcome::Failed { reason } => ("failed", Some(reason.as_str().to_string())),
    };
    ManifestReadinessGateDto {
        kind,
        target,
        status,
        reason,
    }
}

fn describe_gate(gate: &ReadinessGate) -> (&'static str, String) {
    match gate {
        ReadinessGate::MinimumTrajectories(c) => ("minimum_trajectories", c.as_usize().to_string()),
        ReadinessGate::MinimumTaskFamilyCoverage(c) => {
            ("minimum_task_family_coverage", c.as_usize().to_string())
        }
        ReadinessGate::MinimumEvaluationPassRate(r) => {
            ("minimum_evaluation_pass_rate", format!("{:.4}", r.as_f64()))
        }
    }
}
