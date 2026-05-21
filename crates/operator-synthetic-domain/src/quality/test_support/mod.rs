//! Seed corpus fixtures for corpus-quality specifications.
//!
//! These fixtures are the shared v7.2 base used to exercise all 13
//! corpus-quality specs. Any future spec must add its own focused failing
//! snapshot here so quality regressions stay explicit and local.

mod episodes;
mod snapshots;

pub use episodes::{
    episode, episode_bug_investigation, episode_incident_payments_timeout,
    episode_product_planning, episode_smart_writing, episode_software_migration,
};
pub use snapshots::{
    clean_corpus_snapshot, corpus_failing_five_specs, corpus_missing_kernel_forward,
    corpus_with_about_not_in_known, corpus_with_action_failing_mcp_request_shape,
    corpus_with_duplicate_model_facing_rows, corpus_with_ingest_in_read_mode,
    corpus_with_invalid_action_target, corpus_with_target_action_leaked_in_system_prompt,
    corpus_with_trace_lacking_cursor_continuation, corpus_with_train_and_eval_sharing_about,
    corpus_with_unknown_memory_ref, corpus_with_unparseable_row,
    corpus_with_write_lacking_read_before_write, corpus_without_frontier_baseline_recorded,
    duplicate_step_snapshot, full_quality_snapshot, inspect_dataset,
    inspect_snapshot_without_split, snapshot_with_audit,
};
