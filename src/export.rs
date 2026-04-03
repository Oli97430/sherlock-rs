use crate::result::{QueryResult, QueryStatus};
use std::collections::HashMap;

pub fn to_csv(results: &[QueryResult]) -> String {
    let mut wtr = csv::Writer::from_writer(vec![]);
    let _ = wtr.write_record(["Username", "Site", "URL", "Status", "Response Time (ms)"]);
    for r in results {
        let _ = wtr.write_record([
            &r.username,
            &r.site_name,
            &r.site_url,
            r.status.as_str(),
            &r.response_time_ms.map(|t| t.to_string()).unwrap_or_default(),
        ]);
    }
    String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default()
}

pub fn to_txt(results: &[QueryResult]) -> String {
    let mut by_username: HashMap<&str, Vec<&QueryResult>> = HashMap::new();
    for r in results {
        by_username.entry(r.username.as_str()).or_default().push(r);
    }

    // Preserve insertion order
    let mut usernames: Vec<&str> = Vec::new();
    for r in results {
        if !usernames.contains(&r.username.as_str()) {
            usernames.push(&r.username);
        }
    }

    let mut out = String::from("Sherlock-RS — Results\n");
    out.push_str(&"=".repeat(50));
    out.push('\n');

    for username in usernames {
        let user_results = &by_username[username];
        let found: Vec<_> = user_results
            .iter()
            .filter(|r| r.status == QueryStatus::Claimed)
            .collect();

        out.push_str(&format!(
            "\n[{}] — Found on {} site(s):\n",
            username,
            found.len()
        ));
        for r in &found {
            out.push_str(&format!("  [+] {}: {}\n", r.site_name, r.site_url));
        }
    }

    out
}
