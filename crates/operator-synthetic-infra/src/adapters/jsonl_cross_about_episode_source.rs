//! JSONL source for externally authored cross-about count episodes.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use operator_synthetic_domain::episode::cross_about_episode::CrossAboutEpisode;

use crate::dto::cross_about_episode_dto::CrossAboutEpisodeDto;
use crate::errors::cross_about_episode_source_error::CrossAboutEpisodeSourceError;
use crate::mappers::cross_about_episode_mapper::CrossAboutEpisodeMapper;

const ADAPTER: &str = "jsonl_cross_about_episode_source";

#[derive(Debug, Clone)]
pub struct JsonlCrossAboutEpisodeSource {
    path: PathBuf,
    limit: Option<usize>,
}

impl JsonlCrossAboutEpisodeSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            limit: None,
        }
    }

    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = (limit > 0).then_some(limit);
        self
    }

    pub fn read(&self) -> Result<Vec<CrossAboutEpisode>, CrossAboutEpisodeSourceError> {
        let file = File::open(&self.path).map_err(|err| {
            CrossAboutEpisodeSourceError::SourceUnavailable {
                adapter: ADAPTER,
                message: format!("open {}: {err}", self.path.display()),
            }
        })?;
        let reader = BufReader::new(file);
        let mut episodes = Vec::new();
        for (zero_based, line) in reader.lines().enumerate() {
            let line_number = zero_based + 1;
            let line = line.map_err(|err| CrossAboutEpisodeSourceError::InvalidRow {
                adapter: ADAPTER,
                line: line_number,
                message: err.to_string(),
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: CrossAboutEpisodeDto = serde_json::from_str(&line).map_err(|err| {
                CrossAboutEpisodeSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            let episode = CrossAboutEpisodeMapper::to_domain(&dto).map_err(|err| {
                CrossAboutEpisodeSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            episodes.push(episode);
        }
        if episodes.is_empty() {
            return Err(CrossAboutEpisodeSourceError::EmptySource { adapter: ADAPTER });
        }
        if let Some(limit) = self.limit {
            episodes.truncate(limit);
        }
        Ok(episodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "operator-xabout-episode-source-{}-{name}",
            std::process::id()
        ))
    }

    fn fixture_line() -> &'static str {
        r#"{"targets":[{"about":"ctrl-ws-eu","expected_refs":["ctrl-ws-eu:wkshop-01"]},{"about":"ctrl-ws-us","expected_refs":["ctrl-ws-us:wkshop-02"]}],"goal":"Count workshops across EU and US.","initial_window":4,"max_iterations":8,"token_budget":4096}"#
    }

    #[test]
    fn reads_well_formed_jsonl() {
        let path = temp_path("well-formed.jsonl");
        fs::write(&path, format!("{}\n", fixture_line())).unwrap();
        let rows = JsonlCrossAboutEpisodeSource::new(&path).read().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entry_about().as_str(), "ctrl-ws-eu");
        assert_eq!(rows[0].targets().len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn defaults_the_token_budget_when_absent() {
        let path = temp_path("no-budget.jsonl");
        fs::write(
            &path,
            r#"{"targets":[{"about":"ctrl-ws-eu","expected_refs":["ctrl-ws-eu:wkshop-01"]}],"goal":"Count.","initial_window":4,"max_iterations":3}"#,
        )
        .unwrap();
        let rows = JsonlCrossAboutEpisodeSource::new(&path).read().unwrap();
        assert_eq!(rows[0].token_budget(), 8192);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_malformed_line_with_line_number() {
        let path = temp_path("malformed.jsonl");
        fs::write(&path, "{bad}\n").unwrap();
        let err = JsonlCrossAboutEpisodeSource::new(&path).read().unwrap_err();
        assert!(matches!(
            err,
            CrossAboutEpisodeSourceError::InvalidRow { line: 1, .. }
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_empty_file() {
        let path = temp_path("empty.jsonl");
        fs::write(&path, "\n").unwrap();
        let err = JsonlCrossAboutEpisodeSource::new(&path).read().unwrap_err();
        assert!(matches!(
            err,
            CrossAboutEpisodeSourceError::EmptySource { .. }
        ));
        let _ = fs::remove_file(path);
    }
}
