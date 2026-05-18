//! What the external predictor will do: which base model and `LoRA`
//! adapter to load, which dataset to predict over, and where to write
//! the predictions. Domain-typed value object so the port stays
//! filesystem-agnostic and refuses empty / whitespace values at
//! construction.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_training_domain::trainer::base_model_id::BaseModelId;
use operator_training_domain::trainer::output_directory::OutputDirectory;
use operator_training_domain::trainer::trainer_command::TrainerCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictorTarget {
    command: TrainerCommand,
    base_model: BaseModelId,
    adapter_directory: OutputDirectory,
    dataset_path: NonEmptyString,
    output_directory: OutputDirectory,
}

impl PredictorTarget {
    pub fn new(
        command: TrainerCommand,
        base_model: BaseModelId,
        adapter_directory: OutputDirectory,
        dataset_path: impl Into<String>,
        output_directory: OutputDirectory,
    ) -> DomainResult<Self> {
        let dataset_path = NonEmptyString::parse(dataset_path, "predictor_target.dataset_path")?;
        Ok(Self {
            command,
            base_model,
            adapter_directory,
            dataset_path,
            output_directory,
        })
    }

    pub fn command(&self) -> &TrainerCommand {
        &self.command
    }

    pub fn base_model(&self) -> &BaseModelId {
        &self.base_model
    }

    pub fn adapter_directory(&self) -> &OutputDirectory {
        &self.adapter_directory
    }

    pub fn dataset_path(&self) -> &str {
        self.dataset_path.as_str()
    }

    pub fn output_directory(&self) -> &OutputDirectory {
        &self.output_directory
    }
}
