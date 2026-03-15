use std::fs;
use std::path::PathBuf;
use std::process::Command;

const START_MARKER: &str = "<!-- retrieval-benchmark:start -->";
const END_MARKER: &str = "<!-- retrieval-benchmark:end -->";

#[derive(Debug)]
struct MetricLine {
    label: String,
    top1: String,
    top3: String,
    top5: String,
    mrr: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readme_path = repo_root.join("README.md");

    let metrics = run_benchmark(&repo_root)?;
    let block = render_block(&metrics);
    update_readme(&readme_path, &block)?;

    println!("Updated README retrieval benchmark block.");
    Ok(())
}

fn run_benchmark(repo_root: &PathBuf) -> Result<Vec<MetricLine>, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(["test", "benchmark_recall_quality", "--", "--ignored", "--nocapture"])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "benchmark command failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let metrics: Vec<_> = stdout.lines().filter_map(parse_metric_line).collect();

    if metrics.len() != 3 {
        return Err("failed to parse retrieval benchmark output".into());
    }

    Ok(metrics)
}

fn parse_metric_line(line: &str) -> Option<MetricLine> {
    let (label, rest) = line.split_once(": top1=")?;
    if !matches!(
        label,
        "semantic stress (0.7/0.3)" | "lexical heavy (0.7/0.3)" | "lexical heavy (0.2/0.8)"
    ) {
        return None;
    }

    let mut parts = rest.split(", ");
    let top1 = parts.next()?.to_string();
    let top3 = parts.next()?.strip_prefix("top3=")?.to_string();
    let top5 = parts.next()?.strip_prefix("top5=")?.to_string();
    let mrr = parts.next()?.strip_prefix("mrr=")?.to_string();

    Some(MetricLine { label: label.to_string(), top1, top3, top5, mrr })
}

fn render_block(metrics: &[MetricLine]) -> String {
    let semantic =
        metrics.iter().find(|metric| metric.label == "semantic stress (0.7/0.3)").unwrap();
    let lexical_default =
        metrics.iter().find(|metric| metric.label == "lexical heavy (0.7/0.3)").unwrap();
    let lexical_vector =
        metrics.iter().find(|metric| metric.label == "lexical heavy (0.2/0.8)").unwrap();

    [
        START_MARKER.to_string(),
        "Latest benchmark snapshot from `cargo test benchmark_recall_quality -- --ignored --nocapture`:".to_string(),
        String::new(),
        format!(
            "- Semantic stress, `0.7 / 0.3`: Top-1 `{}`, Top-3 `{}`, Top-5 `{}`, MRR `{}`",
            semantic.top1, semantic.top3, semantic.top5, semantic.mrr
        ),
        format!(
            "- Lexical-heavy, `0.7 / 0.3`: Top-1 `{}`, Top-3 `{}`, Top-5 `{}`, MRR `{}`",
            lexical_default.top1, lexical_default.top3, lexical_default.top5, lexical_default.mrr
        ),
        format!(
            "- Lexical-heavy, `0.2 / 0.8`: Top-1 `{}`, Top-3 `{}`, Top-5 `{}`, MRR `{}`",
            lexical_vector.top1, lexical_vector.top3, lexical_vector.top5, lexical_vector.mrr
        ),
        String::new(),
        "Use `cargo run --bin update_retrieval_benchmark_readme` to refresh this block.".to_string(),
        END_MARKER.to_string(),
    ]
    .join("\n")
}

fn update_readme(readme_path: &PathBuf, block: &str) -> Result<(), Box<dyn std::error::Error>> {
    let readme = fs::read_to_string(readme_path)?;
    let start = readme.find(START_MARKER).ok_or("README start marker not found")?;
    let end = readme.find(END_MARKER).ok_or("README end marker not found")? + END_MARKER.len();

    let mut updated = String::new();
    updated.push_str(&readme[..start]);
    updated.push_str(block);
    updated.push_str(&readme[end..]);

    fs::write(readme_path, updated)?;
    Ok(())
}
