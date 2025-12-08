use rusqlite::Connection;
use std::fs;

// Simple structs matching the database schema
#[derive(Debug)]
struct PullRequest {
    id: String,
    title: String,
    description: Option<String>,
    repo: String,
    author: String,
    branch: String,
    created_at: String,
}

#[derive(Debug)]
struct ReviewTask {
    id: String,
    pr_id: String,
    title: String,
    description: String,
    files: String, // JSON string
    stats: String, // JSON string
    insight: Option<String>,
    patches: Option<String>, // JSON string
    diagram: Option<String>,
    ai_generated: bool,
    status: String,
    sub_flow: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Determine database path
    let db_path = if let Ok(path) = std::env::var("LAREVIEW_DB_PATH") {
        std::path::PathBuf::from(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        cwd.join(".lareview").join("db.sqlite")
    };

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    println!("Connecting to database at: {}", db_path.display());

    let conn = Connection::open(&db_path)?;

    // Sample PR that feels like a real production auth hardening change
    let pr = PullRequest {
        id: "pr-1234-auth-hardening".to_string(),
        title: "Harden customer authentication and session handling".to_string(),
        description: Some(
            "Context:\n\
- Follow up to incident SEC-42 where stolen session cookies were reused across devices.\n\
- Align backend auth behavior with the latest mobile and desktop clients.\n\n\
Scope:\n\
- Move session handling to JWT based tokens behind the API gateway.\n\
- Introduce Argon2 password hashing and account lockout after repeated failures.\n\
- Add structured security logging and metrics for authentication failures.\n\n\
Out of scope:\n\
- UI changes to the login form.\n\
- SSO providers other than Google.\n\
- Session management for legacy SOAP APIs.\n"
                .to_string(),
        ),
        repo: "acme/shop-web".to_string(),
        author: "alice".to_string(),
        branch: "feature/auth-hardening".to_string(),
        created_at: "2024-12-08T10:00:00Z".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO pull_requests (id, title, description, repo, author, branch, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&pr.id, &pr.title, &pr.description, &pr.repo, &pr.author, &pr.branch, &pr.created_at),
    )?;
    println!("Inserted PR: {}", pr.title);

    // Sample tasks with sub-flows, modeled after real review items
    let tasks = vec![
        // Authentication and sessions sub-flow
        ReviewTask {
            id: "auth-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review JWT claims and signing strategy".to_string(),
            description: "Review the JWT implementation in src/auth/jwt.rs, the auth middleware, and related security config. Confirm algorithm choice, expiry, issuer and audience validation, and that the key management story is realistic for production (rotation, multiple keys, staging vs prod).".to_string(),
            files: r#"["src/auth/jwt.rs", "src/middleware/auth.rs", "src/config/security.rs"]"#.to_string(),
            stats: r#"{"additions": 120, "deletions": 31, "risk": "HIGH", "tags": ["security", "authentication", "jwt"]}"#.to_string(),
            insight: Some("JWT bugs usually come from validation shortcuts or confusing key management. Treat this as a security review and check how this would behave in a real incident.".to_string()),
            patches: Some(
                r#"
[
  {
    "file": "src/auth/jwt.rs",
    "hunk": "diff --git a/src/auth/jwt.rs b/src/auth/jwt.rs\n--- a/src/auth/jwt.rs\n+++ b/src/auth/jwt.rs\n@@ -1,3 +1,33 @@\n+use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};\n+use serde::{Deserialize, Serialize};\n+use crate::config::security::JwtConfig;\n+\n+#[derive(Debug, Serialize, Deserialize)]\n+pub struct Claims {\n+    pub sub: String,\n+    pub exp: usize,\n+    pub iss: String,\n+    pub aud: String,\n+}\n+\n+pub fn generate_token(user_id: &str, cfg: &JwtConfig) -> Result<String, JwtError> {\n+    let claims = Claims {\n+        sub: user_id.to_string(),\n+        exp: cfg.exp_from_now(),\n+        iss: cfg.issuer.clone(),\n+        aud: cfg.audience.clone(),\n+    };\n+\n+    let header = Header::new(Algorithm::HS256);\n+    // TODO: consider key rotation story before enabling this in production\n+    encode(&header, &claims, &EncodingKey::from_secret(cfg.signing_key.as_bytes()))\n+        .map_err(JwtError::Encode)\n+}\n+\n+pub fn validate_token(token: &str, cfg: &JwtConfig) -> Result<Claims, JwtError> {\n+    let mut validation = Validation::new(Algorithm::HS256);\n+    validation.set_audience(&[cfg.audience.clone()]);\n+    // Note: issuer is not validated yet, review before shipping\n+    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(cfg.signing_key.as_bytes()), &validation)?;\n+    Ok(token_data.claims)\n+}\n"
  },
  {
    "file": "src/middleware/auth.rs",
    "hunk": "diff --git a/src/middleware/auth.rs b/src/middleware/auth.rs\n--- a/src/middleware/auth.rs\n+++ b/src/middleware/auth.rs\n@@ -5,3 +5,23 @@\n+use crate::auth::jwt::validate_token;\n+use crate::config::security::jwt_config;\n+\n+pub fn authenticate(request: &Request) -> Result<UserContext, AppError> {\n+    let token = extract_bearer_token(request)?;\n+    let cfg = jwt_config();\n+    let claims = validate_token(&token, &cfg)?;\n+\n+    // Note: we currently log the first 8 chars of the token for debugging.\n+    // Review if this is acceptable for production logs.\n+    tracing::debug!(\"auth.jwt_valid user_id={} token_prefix={}\", claims.sub, &token[0..8]);\n+\n+    Ok(UserContext {\n+        user_id: claims.sub,\n+        session_token: token,\n+    })\n+}\n"
  },
  {
    "file": "src/config/security.rs",
    "hunk": "diff --git a/src/config/security.rs b/src/config/security.rs\n--- a/src/config/security.rs\n+++ b/src/config/security.rs\n@@ -1,3 +1,18 @@\n+pub struct JwtConfig {\n+    pub signing_key: String,\n+    pub issuer: String,\n+    pub audience: String,\n+    pub default_ttl_seconds: u64,\n+}\n+\n+impl JwtConfig {\n+    pub fn exp_from_now(&self) -> usize {\n+        // naive, for now we trust the system clock\n+        (chrono::Utc::now().timestamp() as u64 + self.default_ttl_seconds) as usize\n+    }\n+}\n+\n+pub fn jwt_config() -> JwtConfig {\n+    // TODO: load from env or secrets manager, not hard coded\n+    JwtConfig { signing_key: \"dev-only-key\".to_string(), issuer: \"acme-auth\".to_string(), audience: \"shop-web\".to_string(), default_ttl_seconds: 900 }\n+}\n"
  }
]
"#
                .trim()
                .to_string(),
            ),
            diagram: None,
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Authentication and sessions".to_string()),
        },
        ReviewTask {
            id: "auth-2".to_string(),
            pr_id: pr.id.clone(),
            title: "Verify login endpoint behavior and error handling".to_string(),
            description: "Review error handling for the login endpoint. Check that we do not leak whether an email exists, that lockout and rate limits behave as expected, and that response shapes are consistent with mobile and web clients. Make sure logs and metrics do not contain secrets.".to_string(),
            files: r#"["src/controllers/auth.rs", "src/services/user_service.rs", "src/rate_limit.rs"]"#.to_string(),
            stats: r#"{"additions": 74, "deletions": 16, "risk": "MEDIUM", "tags": ["authentication", "error-handling", "api"]}"#.to_string(),
            insight: Some("Pay attention to differences between 401, 403, and 429, and confirm that we keep responses generic even when the underlying reason differs.".to_string()),
            patches: Some(
                r#"
[
  {
    "file": "src/controllers/auth.rs",
    "hunk": "diff --git a/src/controllers/auth.rs b/src/controllers/auth.rs\n--- a/src/controllers/auth.rs\n+++ b/src/controllers/auth.rs\n@@ -10,3 +10,24 @@\n+pub async fn login(Json(body): Json<LoginRequest>, Extension(client_ip): Extension<String>, State(svc): State<AuthService>) -> Result<Json<LoginResponse>, AppError> {\n+    if rate_limit::too_many_attempts(&client_ip)? {\n+        // Intentionally do not tell the client that they are rate limited vs invalid credentials\n+        return Err(AppError::TooManyAttempts);\n+    }\n+\n+    match svc.validate_credentials(&body.email, &body.password).await {\n+        Ok(user) => {\n+            let token = svc.issue_token(&user.id).await?;\n+            Ok(Json(LoginResponse { token, user_id: user.id }))\n+        }\n+        Err(_) => {\n+            rate_limit::record_failure(&client_ip)?;\n+            Err(AppError::InvalidCredentials)\n+        }\n+    }\n+}\n"
  },
  {
    "file": "src/services/user_service.rs",
    "hunk": "diff --git a/src/services/user_service.rs b/src/services/user_service.rs\n--- a/src/services/user_service.rs\n+++ b/src/services/user_service.rs\n@@ -30,3 +30,22 @@\n+pub async fn validate_credentials(&self, email: &str, password: &str) -> Result<User, AuthError> {\n+    let user = self.repo.find_active_by_email(email).await.map_err(|_| AuthError::InvalidCredentials)?;\n+\n+    if !passwords::verify(&user.password_hash, password)? {\n+        // Do not reveal which part failed\n+        return Err(AuthError::InvalidCredentials);\n+    }\n+\n+    Ok(user)\n+}\n+\n+pub async fn issue_token(&self, user_id: &str) -> Result<String, AuthError> {\n+    let cfg = self.jwt_cfg.clone();\n+    crate::auth::jwt::generate_token(user_id, &cfg).map_err(AuthError::TokenGeneration)\n+}\n"
  },
  {
    "file": "src/rate_limit.rs",
    "hunk": "diff --git a/src/rate_limit.rs b/src/rate_limit.rs\n--- a/src/rate_limit.rs\n+++ b/src/rate_limit.rs\n@@ -1,3 +1,16 @@\n+pub fn too_many_attempts(client_ip: &str) -> Result<bool, RateLimitError> {\n+    // Backed by Redis in production, in memory in tests\n+    let attempts = backend::get_attempts(client_ip)?;\n+    Ok(attempts >= 5)\n+}\n+\n+pub fn record_failure(client_ip: &str) -> Result<(), RateLimitError> {\n+    backend::increment_attempts(client_ip)\n+}\n"
  }
]
"#
                .trim()
                .to_string(),
            ),
            diagram: None,
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Authentication and sessions".to_string()),
        },

        // Account and profile sub-flow
        ReviewTask {
            id: "profile-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review profile update validation and immutable fields".to_string(),
            description: "Review validation logic for profile updates and rules around immutable fields. Check that user id, email, and email_verified cannot be changed through this endpoint, that PII is sanitized, and that locale and marketing consent behave correctly for GDPR.".to_string(),
            files: r#"["src/controllers/profile.rs", "src/validation.rs", "src/models/user.rs"]"#.to_string(),
            stats: r#"{"additions": 61, "deletions": 19, "risk": "MEDIUM", "tags": ["validation", "security", "gdpr"]}"#.to_string(),
            insight: Some("Profile endpoints tend to accrete many special cases over time. Check for edge cases like null locale, too long names, and attempts to overwrite flags like email_verified.".to_string()),
            patches: Some(
                r#"
[
  {
    "file": "src/controllers/profile.rs",
    "hunk": "diff --git a/src/controllers/profile.rs b/src/controllers/profile.rs\n--- a/src/controllers/profile.rs\n+++ b/src/controllers/profile.rs\n@@ -5,3 +5,20 @@\n+pub async fn update_profile(Authenticated(user): AuthenticatedUser, Json(body): Json<ProfileUpdateRequest>, State(svc): State<ProfileService>) -> Result<Json<ProfileUpdateResponse>, AppError> {\n+    let validated = validate_profile_data(&body).map_err(AppError::Validation)?;\n+\n+    // user.id is taken from the token, not from the body\n+    svc.update_profile(&user.id, validated).await?;\n+\n+    Ok(Json(ProfileUpdateResponse { ok: true }))\n+}\n"
  },
  {
    "file": "src/validation.rs",
    "hunk": "diff --git a/src/validation.rs b/src/validation.rs\n--- a/src/validation.rs\n+++ b/src/validation.rs\n@@ -1,3 +1,24 @@\n+pub fn validate_profile_data(input: &ProfileUpdateRequest) -> Result<ValidProfileUpdate, ValidationError> {\n+    if input.email.is_some() {\n+        // email changes go through a separate flow with verification\n+        return Err(ValidationError::FieldNotAllowed(\"email\".to_string()));\n+    }\n+\n+    let display_name = input.display_name.as_deref().unwrap_or(\"\").trim();\n+    if display_name.len() > 100 {\n+        return Err(ValidationError::TooLong(\"display_name\".to_string()));\n+    }\n+\n+    let locale = input.locale.clone().unwrap_or_else(|| \"en-US\".to_string());\n+\n+    Ok(ValidProfileUpdate { display_name: display_name.to_string(), locale, marketing_consent: input.marketing_consent })\n+}\n"
  },
  {
    "file": "src/models/user.rs",
    "hunk": "diff --git a/src/models/user.rs b/src/models/user.rs\n--- a/src/models/user.rs\n+++ b/src/models/user.rs\n@@ -10,3 +10,10 @@\n+pub struct ValidProfileUpdate {\n+    pub display_name: String,\n+    pub locale: String,\n+    pub marketing_consent: Option<bool>,\n+}\n+\n+// Note: email and email_verified are intentionally not present here\n"
  }
]
"#
                .trim()
                .to_string(),
            ),
            diagram: None,
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Account and profile".to_string()),
        },

        // Security and password policy sub-flow
        ReviewTask {
            id: "security-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Verify password hashing parameters and migration path".to_string(),
            description: "Review password hashing and the proposed migration. Confirm Argon2 parameters, salt handling, and that we can migrate from existing bcrypt hashes without forcing a full password reset event.".to_string(),
            files: r#"["src/auth/password.rs", "src/config/security.rs", "migrations/2024_12_08_rehash_passwords.sql"]"#.to_string(),
            stats: r#"{"additions": 54, "deletions": 7, "risk": "HIGH", "tags": ["security", "password-hashing", "migration"]}"#.to_string(),
            insight: Some("Check both the happy path and migration path. Look for low iteration counts, small memory limits, or error handling that might leak password related information.".to_string()),
            patches: Some(
                r#"
[
  {
    "file": "src/auth/password.rs",
    "hunk": "diff --git a/src/auth/password.rs b/src/auth/password.rs\n--- a/src/auth/password.rs\n+++ b/src/auth/password.rs\n@@ -1,5 +1,28 @@\n-use crate::config::security::ARGON2_CONFIG;\n+use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};\n+use crate::config::security::ARGON2_PARAMS;\n+\n+pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {\n+    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());\n+    let argon2 = Argon2::new(ARGON2_PARAMS.algorithm, ARGON2_PARAMS.version, ARGON2_PARAMS.params);\n+    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;\n+    Ok(password_hash.to_string())\n+}\n+\n+pub fn verify_password(expected_hash: &str, candidate: &str) -> Result<bool, argon2::password_hash::Error> {\n+    let parsed = PasswordHash::new(expected_hash)?;\n+    if parsed.algorithm == \"bcrypt\" {\n+        // Legacy hashes, will be rehashed to Argon2 on next successful login\n+        return Ok(bcrypt::verify(candidate, expected_hash).unwrap_or(false));\n+    }\n+\n+    let argon2 = Argon2::default();\n+    Ok(argon2.verify_password(candidate.as_bytes(), &parsed).is_ok())\n+}\n"
  },
  {
    "file": "src/config/security.rs",
    "hunk": "diff --git a/src/config/security.rs b/src/config/security.rs\n--- a/src/config/security.rs\n+++ b/src/config/security.rs\n@@ -20,3 +20,11 @@\n+pub struct Argon2Params {\n+    pub algorithm: argon2::Algorithm,\n+    pub version: argon2::Version,\n+    pub params: argon2::Params,\n+}\n+\n+pub const ARGON2_PARAMS: Argon2Params = Argon2Params {\n+    algorithm: argon2::Algorithm::Argon2id,\n+    version: argon2::Version::V0x13,\n+    params: argon2::Params::new(65536, 3, 4, None),\n+};\n"
  },
  {
    "file": "migrations/2024_12_08_rehash_passwords.sql",
    "hunk": "diff --git a/migrations/2024_12_08_rehash_passwords.sql b/migrations/2024_12_08_rehash_passwords.sql\n--- a/migrations/2024_12_08_rehash_passwords.sql\n+++ b/migrations/2024_12_08_rehash_passwords.sql\n@@ -1,3 +1,9 @@\n+-- Marker migration only. Actual rehash happens on login.\n+-- Used to track rollout in change management.\n+\n+INSERT INTO password_migration_audit (created_at, note)\n+VALUES (CURRENT_TIMESTAMP, 'Enabled lazy bcrypt to Argon2 migration');\n"
  }
]
"#
                .trim()
                .to_string(),
            ),
            diagram: None,
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Security and password policy".to_string()),
        },

        // Logging and observability (no specific sub-flow)
        ReviewTask {
            id: "logging-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Add authentication security logging and metrics".to_string(),
            description: "Check that we log failed logins, lockouts, and suspicious access in a structured way that SIEM can consume. Logs must not contain raw tokens, passwords, or full PII. Metrics should be tagged in a way that is useful for alerting but still privacy aware.".to_string(),
            files: r#"["src/logging.rs", "src/middleware/auth.rs", "src/telemetry/metrics.rs"]"#.to_string(),
            stats: r#"{"additions": 47, "deletions": 4, "risk": "LOW", "tags": ["logging", "security", "observability"]}"#.to_string(),
            insight: Some("Too much noisy logging hides real incidents. Too little logging makes incident response guesswork. Aim for structured, low cardinality fields that are easy to query.".to_string()),
            patches: Some(
                r#"
[
  {
    "file": "src/middleware/auth.rs",
    "hunk": "diff --git a/src/middleware/auth.rs b/src/middleware/auth.rs\n--- a/src/middleware/auth.rs\n+++ b/src/middleware/auth.rs\n@@ -30,3 +30,18 @@\n+pub fn on_auth_failure(client_ip: &str, reason: &str) {\n+    // Correlate with a request scoped id from tracing\n+    let corr = crate::logging::correlation_id();\n+    tracing::warn!(\"auth.failure correlation_id={} ip={} reason={}\", corr, client_ip, reason);\n+    crate::telemetry::metrics::auth_failure(reason);\n+}\n"
  },
  {
    "file": "src/logging.rs",
    "hunk": "diff --git a/src/logging.rs b/src/logging.rs\n--- a/src/logging.rs\n+++ b/src/logging.rs\n@@ -1,3 +1,10 @@\n+use std::cell::RefCell;\n+\n+thread_local! {\n+    static CORRELATION_ID: RefCell<String> = RefCell::new(String::new());\n+}\n+\n+pub fn correlation_id() -> String {\n+    CORRELATION_ID.with(|id| id.borrow().clone())\n+}\n"
  },
  {
    "file": "src/telemetry/metrics.rs",
    "hunk": "diff --git a/src/telemetry/metrics.rs b/src/telemetry/metrics.rs\n--- a/src/telemetry/metrics.rs\n+++ b/src/telemetry/metrics.rs\n@@ -1,3 +1,12 @@\n+use prometheus::{IntCounterVec, register_int_counter_vec};\n+\n+lazy_static::lazy_static! {\n+    static ref AUTH_FAILURES: IntCounterVec = register_int_counter_vec!(\"auth_failures_total\", \"Authentication failures by reason\", &[\"reason\"]).unwrap();\n+}\n+\n+pub fn auth_failure(reason: &str) {\n+    AUTH_FAILURES.with_label_values(&[reason]).inc();\n+}\n"
  }
]
"#
                .trim()
                .to_string(),
            ),
            diagram: None,
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: None,
        },
    ];

    // Insert all tasks
    for task in tasks {
        conn.execute(
            r#"INSERT OR REPLACE INTO tasks (id, pull_request_id, title, description, files, stats, insight, patches, diagram, ai_generated, status, sub_flow) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            (
                &task.id,
                &task.pr_id,
                &task.title,
                &task.description,
                &task.files,
                &task.stats,
                &task.insight,
                &task.patches,
                &task.diagram,
                if task.ai_generated { 1 } else { 0 },
                &task.status,
                &task.sub_flow,
            ),
        )?;
        println!(
            "Inserted task: {} (Sub-flow: {})",
            task.title,
            task.sub_flow.as_deref().unwrap_or("None")
        );
    }

    println!("\nSample data successfully added to database!");
    println!("Database location: {}", db_path.display());
    println!(
        "Run the application with `cargo run` to see the intent centric layout against this auth hardening PR."
    );

    Ok(())
}
