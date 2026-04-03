use crate::result::{QueryResult, QueryStatus};

pub fn to_csv(results: &[QueryResult]) -> String {
    let mut wtr = csv::Writer::from_writer(vec![]);
    let _ = wtr.write_record(["Site", "URL", "Status", "Response Time (ms)"]);
    for r in results {
        let _ = wtr.write_record([
            &r.site_name,
            &r.site_url,
            r.status.as_str(),
            &r.response_time_ms
                .map(|t| t.to_string())
                .unwrap_or_default(),
        ]);
    }
    String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default()
}

pub fn to_txt(username: &str, results: &[QueryResult]) -> String {
    let mut out = format!("Sherlock-RS Results for: {}\n", username);
    out.push_str(&"=".repeat(50));
    out.push('\n');

    let found: Vec<_> = results
        .iter()
        .filter(|r| r.status == QueryStatus::Claimed)
        .collect();

    out.push_str(&format!("\nFound on {} sites:\n\n", found.len()));

    for r in &found {
        out.push_str(&format!("[+] {}: {}\n", r.site_name, r.site_url));
    }

    out
}
