use std::fs;
use std::process::Command;

#[test]
fn mock_teacher_accepts_regression_pack() {
    let fixture = Fixture::new("accepted");
    fixture.write_scenarios();
    fixture.write_pack();

    let output = Command::new(binary())
        .args(fixture.args())
        .arg("--mock-teacher")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.output.join("trajectories.jsonl").is_file());
    assert!(fixture.output.join("dropped.jsonl").is_file());
    assert!(String::from_utf8_lossy(&output.stdout).contains("gate_passed: true"));
}

#[test]
fn mock_wrong_teacher_persists_predicted_action_in_drop() {
    let fixture = Fixture::new("wrong");
    fixture.write_scenarios();
    fixture.write_pack();

    let output = Command::new(binary())
        .args(fixture.args())
        .arg("--mock-wrong")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let dropped = fs::read_to_string(fixture.output.join("dropped.jsonl")).unwrap();
    assert!(dropped.contains("\"predicted_action\""));
    assert!(dropped.contains("\"subject_hash\""));
    assert!(dropped.contains("\"teacher_finish_reason\":\"stop\""));
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_operator-regression-pack-v7")
}

#[derive(Debug)]
struct Fixture {
    root: std::path::PathBuf,
    scenarios: std::path::PathBuf,
    pack: std::path::PathBuf,
    output: std::path::PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "operator-regression-pack-cli-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            scenarios: root.join("scenarios.jsonl"),
            pack: root.join("regression_pack_v7.txt"),
            output: root.join("out"),
            root,
        }
    }

    fn args(&self) -> Vec<String> {
        vec![
            "--scenarios".to_string(),
            self.scenarios.display().to_string(),
            "--pack".to_string(),
            self.pack.display().to_string(),
            "--output".to_string(),
            self.output.display().to_string(),
        ]
    }

    fn write_scenarios(&self) {
        fs::write(&self.scenarios, scenario_line()).unwrap();
    }

    fn write_pack(&self) {
        fs::write(
            &self.pack,
            "# regression pack\nscenario:stop:no-candidate:0028\n",
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn scenario_line() -> String {
    r#"{"scenario_id":"scenario:stop:no-candidate:0028","target":"stop","acceptance_criteria":{"expected_stop_reason":"no_candidate"},"subject":{"about":"about:migration:stop:no-candidate:case-028","mode":"read","task_family":"realistic.stop.no-candidate","goal":"Tools have been exhausted on this about; remaining budget would not produce a new ref that changes the answer.","allowed_tools":["kernel_wake","kernel_ask","kernel_near","kernel_goto","kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"],"visible_state":{"known_refs":["about:migration:stop:no-candidate:case-028:node:state:000"],"known_dimensions":[],"budget":{"calls_remaining":0,"tokens_remaining":600}}}}"#.to_string()
}
