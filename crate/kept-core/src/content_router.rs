/// Content-aware compression mode router — ported from SQZ confidence_router.rs
/// Source: D:\01work\Active\references\campbellr\sqz\sqz_engine\src\confidence_router.rs
///
/// Analyzes input content and selects compression aggressiveness:
/// - High-risk (stack traces, configs, secrets, legal) → Safe
/// - Low-entropy repetitive content → Aggressive
/// - Normal content → Default

/// Compression mode selected by the router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionMode {
    /// Minimal compression — preserve all structure (stack traces, configs, legal)
    Safe,
    /// Balanced compression
    Default,
    /// Maximum compression — repetitive/boilerplate content
    Aggressive,
}

impl AdmissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Default => "default",
            Self::Aggressive => "aggressive",
        }
    }
}

/// Routes content to the appropriate admission mode.
pub struct ContentRouter;

impl Default for ContentRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentRouter {
    pub fn new() -> Self {
        Self
    }

    /// Analyze content and return the recommended admission mode.
    pub fn route(&self, content: &str) -> AdmissionMode {
        if content.len() < 100 {
            return AdmissionMode::Default;
        }

        // High-risk patterns → always Safe
        if self.is_high_risk(content) {
            return AdmissionMode::Safe;
        }

        // Shannon entropy heuristic
        let entropy = shannon_entropy(content);

        // Very low entropy → mostly boilerplate → aggressive
        if entropy < 2.5 {
            return AdmissionMode::Aggressive;
        }

        AdmissionMode::Default
    }

    /// High-risk content that must never be aggressively compressed.
    fn is_high_risk(&self, content: &str) -> bool {
        let lower = content.to_lowercase();

        // Stack traces — require structural markers
        if (lower.contains("panicked at") || lower.contains("traceback"))
            && (lower.contains("stack backtrace")
                || lower.contains("  0:")
                || lower.contains("  at "))
        {
            return true;
        }
        if lower.contains("stack trace") && lower.contains("  at ") {
            return true;
        }

        // Database migrations — require SQL keywords
        if (lower.contains("alter table")
            || lower.contains("create table")
            || lower.contains("drop table"))
            && (lower.contains("column")
                || lower.contains("index")
                || lower.contains("constraint")
                || lower.contains("primary key")
                || lower.contains("references"))
        {
            return true;
        }

        // PEM headers always high-risk
        if lower.contains("-----begin") {
            return true;
        }

        // Security configs — require structural context
        if (lower.contains("private_key")
            || lower.contains("secret_key")
            || lower.contains("api_key"))
            && (lower.contains('=') || lower.contains(':'))
            && !looks_like_commit_log(&lower)
        {
            return true;
        }
        if lower.contains("password")
            && (lower.contains("password:")
                || lower.contains("password=")
                || lower.contains("password \""))
            && !looks_like_commit_log(&lower)
        {
            return true;
        }

        // Legal/compliance
        if lower.contains("terms of service")
            || lower.contains("privacy policy")
            || lower.contains("license agreement")
            || lower.contains("gdpr")
        {
            return true;
        }

        // Kubernetes secrets/configmaps
        if lower.contains("apiversion:")
            && lower.contains("kind:")
            && (lower.contains("secret") || lower.contains("configmap"))
        {
            return true;
        }

        false
    }
}

/// Heuristic: does this content look like a git commit log?
fn looks_like_commit_log(lower: &str) -> bool {
    let lines: Vec<&str> = lower.lines().take(10).collect();
    let mut commit_lines = 0;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.len() > 8 && trimmed[..7].chars().all(|c| c.is_ascii_hexdigit()) {
            commit_lines += 1;
        }
        if trimmed.starts_with("feat:")
            || trimmed.starts_with("fix:")
            || trimmed.starts_with("chore:")
            || trimmed.starts_with("docs:")
            || trimmed.starts_with("refactor:")
            || trimmed.starts_with("test:")
            || trimmed.starts_with("ci:")
            || trimmed.starts_with("style:")
            || trimmed.starts_with("perf:")
        {
            commit_lines += 1;
        }
    }
    commit_lines > 0 && commit_lines >= lines.len() / 3
}

/// Shannon entropy in bits per character.
fn shannon_entropy(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let mut freq = [0u64; 256];
    for &byte in text.as_bytes() {
        freq[byte as usize] += 1;
    }
    let len = text.len() as f64;
    let mut entropy = 0.0;
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_trace_routes_safe() {
        let router = ContentRouter::new();
        let trace = "thread 'main' panicked at 'index out of bounds', src/main.rs:42\nstack backtrace:\n  0: std::panicking::begin_panic\n  1: myapp::process\n  2: main";
        assert_eq!(router.route(trace), AdmissionMode::Safe);
    }

    #[test]
    fn migration_routes_safe() {
        let router = ContentRouter::new();
        let migration = "ALTER TABLE users ADD COLUMN email VARCHAR(255) NOT NULL;\nCREATE TABLE sessions (id UUID PRIMARY KEY, user_id INT REFERENCES users(id));";
        assert_eq!(router.route(migration), AdmissionMode::Safe);
    }

    #[test]
    fn git_log_not_high_risk() {
        let router = ContentRouter::new();
        let git_log = "abc1234 fix: password reset flow\ndef5678 feat: migrate user auth to OAuth\n1234567 chore: update dependencies\n8901234 fix: migration path handling\nabcdef0 refactor: clean up error handling\n1111111 docs: update README\n2222222 fix: handle edge case in parser\n3333333 feat: add new API endpoint";
        assert_ne!(router.route(git_log), AdmissionMode::Safe);
    }

    #[test]
    fn password_config_routes_safe() {
        let router = ContentRouter::new();
        let config = "database:\n  host: localhost\n  port: 5432\n  username: admin\n  password: super_secret_123\n  database: myapp_prod\n  ssl: true\n  pool_size: 10";
        assert_eq!(router.route(config), AdmissionMode::Safe);
    }

    #[test]
    fn normal_code_routes_default() {
        let router = ContentRouter::new();
        let code = "fn process(items: &[Item]) -> Result<Vec<Output>, Error> {\n    let mut results = Vec::new();\n    for item in items {\n        let output = transform(item)?;\n        results.push(output);\n    }\n    Ok(results)\n}";
        assert_eq!(router.route(code), AdmissionMode::Default);
    }

    #[test]
    fn short_content_is_default() {
        let router = ContentRouter::new();
        assert_eq!(router.route("hello"), AdmissionMode::Default);
    }

    #[test]
    fn pem_key_routes_safe() {
        let router = ContentRouter::new();
        let key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF8PbnGy0AHB7MhgHcTz6sE2I2yPB\nnot-a-real-key-but-long-enough-to-trigger-the-router-safely-at-over-100-chars";
        assert_eq!(router.route(key), AdmissionMode::Safe);
    }

    #[test]
    fn shannon_entropy_basic() {
        assert_eq!(shannon_entropy(""), 0.0);
        assert!(shannon_entropy("aaaa") < shannon_entropy("abcd"));
    }
}
