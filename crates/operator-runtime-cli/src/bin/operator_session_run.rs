use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "operator-session-run")]
#[command(about = "Run single-step Operator runtime replay sessions")]
struct Cli {
    #[arg(long)]
    scenario_jsonl: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value = "https://0.5b.llm.underpassai.com/v1")]
    operator_endpoint: String,
    #[arg(long, default_value = "operator-v8.1.2")]
    operator_adapter_id: String,
    #[arg(long)]
    operator_client_cert: Option<PathBuf>,
    #[arg(long)]
    operator_client_key: Option<PathBuf>,
    #[arg(long)]
    #[arg(alias = "kmp-grpc-endpoint")]
    kmp_mcp_endpoint: String,
    #[arg(long, default_value = "read")]
    mode: String,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    filter_tools: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    eprintln!(
        "operator-session-run configured scenario_jsonl={} output_dir={} operator_endpoint={} adapter={} kmp_mcp_endpoint={} mode={} limit={} filter_tools={}",
        cli.scenario_jsonl.display(),
        cli.output_dir.display(),
        cli.operator_endpoint,
        cli.operator_adapter_id,
        cli.kmp_mcp_endpoint,
        cli.mode,
        cli.limit
            .map_or_else(|| "none".to_string(), |value| value.to_string()),
        cli.filter_tools.as_deref().unwrap_or("none")
    );
    if let Some(cert) = &cli.operator_client_cert {
        eprintln!("operator client cert configured at {}", cert.display());
    }
    if let Some(key) = &cli.operator_client_key {
        eprintln!("operator client key configured at {}", key.display());
    }
    eprintln!("Track B build stops before live endpoint execution.");
}
