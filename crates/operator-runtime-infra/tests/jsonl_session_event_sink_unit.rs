use std::fs;
use std::io::Write;
use std::path::PathBuf;

use operator_runtime_infra::adapters::jsonl_session_event_sink::JsonlSessionEventSink;

#[test]
fn opening_jsonl_sink_preserves_existing_evidence() {
    let path = temp_path("jsonl-sink-append.jsonl");
    {
        let mut file = fs::File::create(&path).expect("create existing evidence file");
        writeln!(file, "{{\"event\":\"existing\"}}").expect("write existing evidence");
    }

    let _sink = JsonlSessionEventSink::new(
        &path,
        "https://operator.example.test/v1",
        "stdio://kernel",
        "sha",
    )
    .expect("sink opens");

    let contents = fs::read_to_string(&path).expect("read evidence file");
    assert!(contents.contains("\"event\":\"existing\""));
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("operator-runtime-{name}-{}", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}
