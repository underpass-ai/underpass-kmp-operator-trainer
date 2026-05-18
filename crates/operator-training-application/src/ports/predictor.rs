//! Port: invoke an external predictor that loads a base model +
//! `LoRA` adapter and emits predictions for an SFT-shaped dataset.
//! Returns a typed `PredictorOutcome` describing where the artefacts
//! landed. Adapters that implement this trait live in
//! `operator-training-infra`.

use crate::errors::predictor_error::PredictorError;
use crate::ports::predictor_outcome::PredictorOutcome;
use crate::ports::predictor_target::PredictorTarget;

pub trait Predictor: std::fmt::Debug + Send + Sync {
    fn predict(&self, target: &PredictorTarget) -> Result<PredictorOutcome, PredictorError>;
}
