//! JSONL source for externally authored window-expansion episodes.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use operator_synthetic_domain::episode::window_expansion_episode::WindowExpansionEpisode;

use crate::dto::window_expansion_episode_dto::WindowExpansionEpisodeDto;
use crate::errors::window_expansion_episode_source_error::WindowExpansionEpisodeSourceError;
use crate::mappers::window_expansion_episode_mapper::WindowExpansionEpisodeMapper;

const ADAPTER: &str = "jsonl_window_expansion_episode_source";

#[derive(Debug, Clone)]
pub struct JsonlWindowExpansionEpisodeSource {
    path: PathBuf,
    limit: Option<usize>,
}

impl JsonlWindowExpansionEpisodeSource {
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

    pub fn read(&self) -> Result<Vec<WindowExpansionEpisode>, WindowExpansionEpisodeSourceError> {
        let file = File::open(&self.path).map_err(|err| {
            WindowExpansionEpisodeSourceError::SourceUnavailable {
                adapter: ADAPTER,
                message: format!("open {}: {err}", self.path.display()),
            }
        })?;
        let reader = BufReader::new(file);
        let mut episodes = Vec::new();
        for (zero_based, line) in reader.lines().enumerate() {
            let line_number = zero_based + 1;
            let line = line.map_err(|err| WindowExpansionEpisodeSourceError::InvalidRow {
                adapter: ADAPTER,
                line: line_number,
                message: err.to_string(),
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: WindowExpansionEpisodeDto = serde_json::from_str(&line).map_err(|err| {
                WindowExpansionEpisodeSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            let episode = WindowExpansionEpisodeMapper::to_domain(&dto).map_err(|err| {
                WindowExpansionEpisodeSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            episodes.push(episode);
        }
        if episodes.is_empty() {
            return Err(WindowExpansionEpisodeSourceError::EmptySource { adapter: ADAPTER });
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
            "operator-window-episode-source-{}-{name}",
            std::process::id()
        ))
    }

    fn fixture_line() -> &'static str {
        r#"{"about":"about:period","goal":"Count workshops across the last four months.","initial_window":8,"max_iterations":5,"token_budget":4096}"#
    }

    #[test]
    fn reads_well_formed_jsonl() {
        let path = temp_path("well-formed.jsonl");
        fs::write(&path, format!("{}\n", fixture_line())).unwrap();
        let rows = JsonlWindowExpansionEpisodeSource::new(&path)
            .read()
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].about().as_str(), "about:period");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn defaults_the_token_budget_when_absent() {
        let path = temp_path("no-budget.jsonl");
        fs::write(
            &path,
            r#"{"about":"about:period","goal":"Count.","initial_window":4,"max_iterations":3}"#,
        )
        .unwrap();
        let rows = JsonlWindowExpansionEpisodeSource::new(&path)
            .read()
            .unwrap();
        assert_eq!(rows[0].token_budget(), 8192);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn truncates_to_the_limit() {
        let path = temp_path("limited.jsonl");
        fs::write(&path, format!("{0}\n{0}\n{0}\n", fixture_line())).unwrap();
        let rows = JsonlWindowExpansionEpisodeSource::new(&path)
            .with_limit(2)
            .read()
            .unwrap();
        assert_eq!(rows.len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_malformed_line_with_line_number() {
        let path = temp_path("malformed.jsonl");
        fs::write(&path, "{bad}\n").unwrap();
        let err = JsonlWindowExpansionEpisodeSource::new(&path)
            .read()
            .unwrap_err();
        assert!(matches!(
            err,
            WindowExpansionEpisodeSourceError::InvalidRow { line: 1, .. }
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_empty_file() {
        let path = temp_path("empty.jsonl");
        fs::write(&path, "\n").unwrap();
        let err = JsonlWindowExpansionEpisodeSource::new(&path)
            .read()
            .unwrap_err();
        assert!(matches!(
            err,
            WindowExpansionEpisodeSourceError::EmptySource { .. }
        ));
        let _ = fs::remove_file(path);
    }
}
