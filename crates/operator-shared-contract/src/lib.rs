//! Operator: shared bounded context — serde DTOs.
//!
//! Each DTO is pure data with `Serialize` + `Deserialize`. Schema docs live
//! in `docs/architecture/operator/shared/contract/README.md`. Mappers in
//! `operator-shared-infra` translate these DTOs to and from
//! `operator-shared-domain` types.

pub mod budget_snapshot_dto;
pub mod cursor_dto;
pub mod escalate_action_dto;
pub mod operator_action_dto;
pub mod per_tool;
pub mod stop_action_dto;
pub mod tool_arguments_dto;
pub mod tool_call_action_dto;
pub mod training_trajectory_dto;
pub mod visible_state_dto;
