//! What the external trainer will do: which command to invoke, which
//! base model to fine-tune, and where to put the output.

use crate::trainer::base_model_id::BaseModelId;
use crate::trainer::output_directory::OutputDirectory;
use crate::trainer::trainer_command::TrainerCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerTarget {
    command: TrainerCommand,
    base_model: BaseModelId,
    output_directory: OutputDirectory,
}

impl TrainerTarget {
    pub fn new(
        command: TrainerCommand,
        base_model: BaseModelId,
        output_directory: OutputDirectory,
    ) -> Self {
        Self {
            command,
            base_model,
            output_directory,
        }
    }

    pub fn command(&self) -> &TrainerCommand {
        &self.command
    }

    pub fn base_model(&self) -> &BaseModelId {
        &self.base_model
    }

    pub fn output_directory(&self) -> &OutputDirectory {
        &self.output_directory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_components() {
        let t = TrainerTarget::new(
            TrainerCommand::parse("sft-trainer").unwrap(),
            BaseModelId::parse("Qwen/Qwen2.5-1.5B-Instruct").unwrap(),
            OutputDirectory::parse("out/run").unwrap(),
        );
        assert_eq!(t.command().as_str(), "sft-trainer");
        assert_eq!(t.base_model().as_str(), "Qwen/Qwen2.5-1.5B-Instruct");
        assert_eq!(t.output_directory().as_str(), "out/run");
    }
}
