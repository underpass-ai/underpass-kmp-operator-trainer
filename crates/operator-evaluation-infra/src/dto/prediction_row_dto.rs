//! Wire shape of one line in `predictions.jsonl` as emitted by
//! `predict_operator_sft.py`:
//!
//! ```json
//! {"step_id": "...", "action": { ...OperatorActionDto... }}
//! ```
//!
//! The `action` field uses the operator action DTO defined in
//! `operator-shared-contract`, so the same mapper that turns wire
//! actions into domain `OperatorAction` values applies here.

use operator_shared_contract::operator_action_dto::OperatorActionDto;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PredictionRowDto {
    pub step_id: String,
    pub action: OperatorActionDto,
}
