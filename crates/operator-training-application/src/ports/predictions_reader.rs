//! Port: read a file (or stream) of step-keyed predictions into typed
//! domain values. The use case never sees the bytes or JSON; the
//! adapter owns the wire format.

use operator_evaluation_domain::prediction::predictions_read_outcome::PredictionsReadOutcome;

use crate::errors::predictions_read_error::PredictionsReadError;

pub trait PredictionsReader: std::fmt::Debug + Send + Sync {
    fn read(&self) -> Result<PredictionsReadOutcome, PredictionsReadError>;
}
