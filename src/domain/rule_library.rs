//! Rule Library - Predefined rules users can easily add
//!
//! This module provides a library of common review rules
//! that users can quickly add to their configuration.

use serde::{Deserialize, Serialize};

/// A template rule from the library that can be added to a user's rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryRule {
    /// Unique identifier for this library rule
    pub id: String,
    /// Display name
    pub name: String,
    /// Category for grouping in the UI
    pub library_category: LibraryCategory,
    /// Category name for rules (e.g., "security", "error-handling")
    pub category: Option<String>,
    /// Description shown in the library
    pub description: String,
    /// The actual rule text that will be used in reviews
    pub text: String,
    /// Optional glob pattern for file matching
    pub glob: Option<String>,
    /// Tags for filtering/searching
    pub tags: Vec<String>,
}

/// Categories for organizing rules in the library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryCategory {
    /// Security-focused rules
    Security,
    /// Code quality and best practices
    CodeQuality,
    /// Testing requirements
    Testing,
    /// Documentation standards
    Documentation,
    /// Performance considerations
    Performance,
    /// API design and compatibility
    ApiDesign,
    /// Language-specific rules
    LanguageSpecific,
    /// Framework-specific rules
    FrameworkSpecific,
}

impl std::fmt::Display for LibraryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Security => write!(f, "Security"),
            Self::CodeQuality => write!(f, "Code Quality"),
            Self::Testing => write!(f, "Testing"),
            Self::Documentation => write!(f, "Documentation"),
            Self::Performance => write!(f, "Performance"),
            Self::ApiDesign => write!(f, "API Design"),
            Self::LanguageSpecific => write!(f, "Language Specific"),
            Self::FrameworkSpecific => write!(f, "Framework Specific"),
        }
    }
}

impl LibraryRule {
    /// Returns all available library rules
    pub fn all() -> Vec<Self> {
        let mut rules = Vec::new();
        rules.extend(Self::security_rules());
        rules.extend(Self::code_quality_rules());
        rules.extend(Self::testing_rules());
        rules.extend(Self::performance_rules());
        rules.extend(Self::api_design_rules());
        rules.extend(Self::language_specific_rules());
        rules.extend(Self::framework_specific_rules());
        rules
    }

    /// Security-focused rules
    fn security_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-security-auth-check".to_string(),
                name: "Authentication Checks".to_string(),
                library_category: LibraryCategory::Security,
                category: Some("security".to_string()),
                description: "Verify all endpoints have proper authentication".to_string(),
                text: "Verify that all new or modified API endpoints have appropriate authentication checks. Flag any endpoint that handles sensitive data without authentication middleware.".to_string(),
                glob: Some("**/*.{rs,ts,js,py,go}".to_string()),
                tags: vec!["security".to_string(), "auth".to_string(), "api".to_string()],
            },
            Self {
                id: "lib-security-secrets".to_string(),
                name: "No Hardcoded Secrets".to_string(),
                library_category: LibraryCategory::Security,
                category: Some("security".to_string()),
                description: "Check for hardcoded API keys, passwords, or tokens".to_string(),
                text: "Check for hardcoded secrets including API keys, passwords, tokens, and credentials. These should be loaded from environment variables or a secrets manager.".to_string(),
                glob: None,
                tags: vec!["security".to_string(), "secrets".to_string()],
            },
            Self {
                id: "lib-security-injection".to_string(),
                name: "Injection Prevention".to_string(),
                library_category: LibraryCategory::Security,
                category: Some("security".to_string()),
                description: "Verify inputs are properly sanitized".to_string(),
                text: "Check that user inputs are properly validated and sanitized before use in SQL queries, shell commands, or HTML output. Look for potential SQL injection, command injection, and XSS vulnerabilities.".to_string(),
                glob: None,
                tags: vec!["security".to_string(), "injection".to_string(), "xss".to_string(), "sql".to_string()],
            },
            Self {
                id: "lib-security-sensitive-data".to_string(),
                name: "Sensitive Data Handling".to_string(),
                library_category: LibraryCategory::Security,
                category: Some("privacy".to_string()),
                description: "Ensure PII and sensitive data are handled securely".to_string(),
                text: "Sensitive data (PII, financial data, health records) should be encrypted at rest and in transit. Avoid logging sensitive data. Ensure proper access controls are in place.".to_string(),
                glob: None,
                tags: vec!["security".to_string(), "pii".to_string(), "privacy".to_string()],
            },
        ]
    }

    /// Code quality rules
    fn code_quality_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-quality-error-handling".to_string(),
                name: "Proper Error Handling".to_string(),
                library_category: LibraryCategory::CodeQuality,
                category: Some("error-handling".to_string()),
                description: "Verify errors are handled gracefully".to_string(),
                text: "Check that errors are properly handled with meaningful error messages. Avoid swallowing errors silently. Ensure async operations have proper error handling and don't leave promises unhandled.".to_string(),
                glob: None,
                tags: vec!["quality".to_string(), "errors".to_string()],
            },
            Self {
                id: "lib-quality-logging".to_string(),
                name: "Appropriate Logging".to_string(),
                library_category: LibraryCategory::CodeQuality,
                category: Some("logging".to_string()),
                description: "Ensure adequate logging for debugging and monitoring".to_string(),
                text: "Important operations should have appropriate logging at the right level (debug, info, warn, error). Error paths should log enough context for debugging. Avoid excessive logging that could impact performance.".to_string(),
                glob: None,
                tags: vec!["quality".to_string(), "logging".to_string(), "observability".to_string()],
            },
            Self {
                id: "lib-quality-naming".to_string(),
                name: "Clear Naming Conventions".to_string(),
                library_category: LibraryCategory::CodeQuality,
                category: Some("naming".to_string()),
                description: "Variable and function names should be descriptive".to_string(),
                text: "Names should be descriptive and follow project conventions. Avoid single-letter variables except for loop indices. Function names should describe what they do. Boolean variables should be named as questions (isValid, hasPermission).".to_string(),
                glob: None,
                tags: vec!["quality".to_string(), "naming".to_string(), "readability".to_string()],
            },
            Self {
                id: "lib-quality-complexity".to_string(),
                name: "Manage Complexity".to_string(),
                library_category: LibraryCategory::CodeQuality,
                category: Some("complexity".to_string()),
                description: "Functions should be focused and not overly complex".to_string(),
                text: "Functions should do one thing well. If a function is longer than ~50 lines or has deep nesting, consider refactoring. High cyclomatic complexity (many branches) makes code hard to test and maintain.".to_string(),
                glob: None,
                tags: vec!["quality".to_string(), "complexity".to_string(), "refactoring".to_string()],
            },
        ]
    }

    /// Testing rules
    fn testing_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-testing-coverage".to_string(),
                name: "Test Coverage for New Code".to_string(),
                library_category: LibraryCategory::Testing,
                category: Some("test-coverage".to_string()),
                description: "New features and bug fixes should have tests".to_string(),
                text: "New features should have unit tests covering the happy path and key error cases. Bug fixes should include a regression test that would have caught the bug. Security-critical code must have thorough test coverage.".to_string(),
                glob: None,
                tags: vec!["testing".to_string(), "coverage".to_string()],
            },
            Self {
                id: "lib-testing-edge-cases".to_string(),
                name: "Edge Case Testing".to_string(),
                library_category: LibraryCategory::Testing,
                category: Some("edge-cases".to_string()),
                description: "Tests should cover boundary conditions and edge cases".to_string(),
                text: "Tests should include edge cases: empty inputs, null/undefined values, boundary values (0, -1, MAX_INT), very long strings, special characters, and concurrent access scenarios where applicable.".to_string(),
                glob: Some("**/*.{test,spec}.{ts,js,rs,py,go}".to_string()),
                tags: vec!["testing".to_string(), "edge-cases".to_string()],
            },
            Self {
                id: "lib-testing-mocking".to_string(),
                name: "Appropriate Test Isolation".to_string(),
                library_category: LibraryCategory::Testing,
                category: Some("test-isolation".to_string()),
                description: "Tests should be isolated and not depend on external services".to_string(),
                text: "Unit tests should mock external dependencies (databases, APIs, file system). Integration tests should use test fixtures or containers. Avoid tests that depend on network availability or shared state.".to_string(),
                glob: Some("**/*.{test,spec}.{ts,js,rs,py,go}".to_string()),
                tags: vec!["testing".to_string(), "mocking".to_string(), "isolation".to_string()],
            },
        ]
    }

    /// Performance rules
    fn performance_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-perf-n-plus-one".to_string(),
                name: "N+1 Query Detection".to_string(),
                library_category: LibraryCategory::Performance,
                category: Some("performance".to_string()),
                description: "Watch for N+1 query patterns in database code".to_string(),
                text: "Look for N+1 query patterns where a query is executed inside a loop. Use eager loading, batch fetching, or joins instead. Each additional query adds latency.".to_string(),
                glob: Some("**/*.{rs,ts,js,py,go,rb}".to_string()),
                tags: vec!["performance".to_string(), "database".to_string(), "queries".to_string()],
            },
            Self {
                id: "lib-perf-pagination".to_string(),
                name: "Pagination for Lists".to_string(),
                library_category: LibraryCategory::Performance,
                category: Some("pagination".to_string()),
                description: "Large lists should be paginated".to_string(),
                text: "Endpoints returning lists should implement pagination. Loading unbounded lists can cause memory issues and slow response times. Use cursor-based pagination for large datasets.".to_string(),
                glob: None,
                tags: vec!["performance".to_string(), "pagination".to_string(), "api".to_string()],
            },
            Self {
                id: "lib-perf-caching".to_string(),
                name: "Cache Expensive Operations".to_string(),
                library_category: LibraryCategory::Performance,
                category: Some("caching".to_string()),
                description: "Consider caching for expensive computations or queries".to_string(),
                text: "Expensive operations (complex queries, external API calls, heavy computations) should be cached when the data doesn't need to be real-time. Consider cache invalidation strategies.".to_string(),
                glob: None,
                tags: vec!["performance".to_string(), "caching".to_string()],
            },
        ]
    }

    /// API design rules
    fn api_design_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-api-breaking-changes".to_string(),
                name: "Breaking Change Detection".to_string(),
                library_category: LibraryCategory::ApiDesign,
                category: Some("breaking-changes".to_string()),
                description: "Identify changes that could break existing clients".to_string(),
                text: "Flag any changes that could break existing API consumers: removed endpoints/fields, changed response structures, modified required parameters, or changed authentication requirements. These require versioning or migration plans.".to_string(),
                glob: None,
                tags: vec!["api".to_string(), "breaking-changes".to_string(), "compatibility".to_string()],
            },
            Self {
                id: "lib-api-versioning".to_string(),
                name: "API Versioning".to_string(),
                library_category: LibraryCategory::ApiDesign,
                category: Some("api-versioning".to_string()),
                description: "API changes should follow versioning strategy".to_string(),
                text: "Breaking API changes should increment the API version. Deprecate old endpoints before removing them. Provide migration guides for breaking changes.".to_string(),
                glob: Some("**/api/**/*.{rs,ts,js,py,go}".to_string()),
                tags: vec!["api".to_string(), "versioning".to_string()],
            },
            Self {
                id: "lib-api-validation".to_string(),
                name: "Input Validation".to_string(),
                library_category: LibraryCategory::ApiDesign,
                category: Some("input-validation".to_string()),
                description: "API inputs should be validated".to_string(),
                text: "All API inputs should be validated for type, format, and business rules. Return clear error messages indicating what validation failed. Use a validation library rather than ad-hoc checks.".to_string(),
                glob: None,
                tags: vec!["api".to_string(), "validation".to_string(), "input".to_string()],
            },
        ]
    }

    /// Language-specific rules
    fn language_specific_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-rust-unwrap".to_string(),
                name: "Rust: Avoid Unwrap in Production".to_string(),
                library_category: LibraryCategory::LanguageSpecific,
                category: Some("rust-safety".to_string()),
                description: "Use proper error handling instead of unwrap()".to_string(),
                text: "Avoid using .unwrap() and .expect() in production code paths. Use proper error handling with ? operator or match statements. Unwrap is acceptable in tests and when the invariant is guaranteed.".to_string(),
                glob: Some("**/*.rs".to_string()),
                tags: vec!["rust".to_string(), "error-handling".to_string()],
            },
            Self {
                id: "lib-ts-any".to_string(),
                name: "TypeScript: Avoid 'any' Type".to_string(),
                library_category: LibraryCategory::LanguageSpecific,
                category: Some("typescript-types".to_string()),
                description: "Use specific types instead of 'any'".to_string(),
                text: "Avoid using the 'any' type as it defeats TypeScript's type safety. Use 'unknown' for truly unknown types and narrow with type guards. Use generics or union types for flexible typing.".to_string(),
                glob: Some("**/*.{ts,tsx}".to_string()),
                tags: vec!["typescript".to_string(), "types".to_string()],
            },
            Self {
                id: "lib-py-type-hints".to_string(),
                name: "Python: Use Type Hints".to_string(),
                library_category: LibraryCategory::LanguageSpecific,
                category: Some("python-types".to_string()),
                description: "Add type hints for function parameters and returns".to_string(),
                text: "Use type hints for function parameters and return types. This improves IDE support, catches bugs early, and serves as documentation. Use mypy or pyright for type checking.".to_string(),
                glob: Some("**/*.py".to_string()),
                tags: vec!["python".to_string(), "types".to_string()],
            },
        ]
    }

    /// Framework-specific rules
    fn framework_specific_rules() -> Vec<Self> {
        vec![
            Self {
                id: "lib-react-hooks".to_string(),
                name: "React: Hooks Best Practices".to_string(),
                library_category: LibraryCategory::FrameworkSpecific,
                category: Some("react-hooks".to_string()),
                description: "Follow React hooks rules and best practices".to_string(),
                text: "Follow the Rules of Hooks: only call hooks at the top level, only call from React functions. Include all dependencies in useEffect/useMemo/useCallback dependency arrays. Avoid infinite loops.".to_string(),
                glob: Some("**/*.{tsx,jsx}".to_string()),
                tags: vec!["react".to_string(), "hooks".to_string()],
            },
            Self {
                id: "lib-react-keys".to_string(),
                name: "React: Proper List Keys".to_string(),
                library_category: LibraryCategory::FrameworkSpecific,
                category: Some("react-keys".to_string()),
                description: "Use stable, unique keys for list items".to_string(),
                text: "List items must have stable, unique keys. Don't use array index as key for lists that can reorder. Keys should be derived from the data itself (id, slug).".to_string(),
                glob: Some("**/*.{tsx,jsx}".to_string()),
                tags: vec!["react".to_string(), "lists".to_string()],
            },
            Self {
                id: "lib-nextjs-data-fetching".to_string(),
                name: "Next.js: Data Fetching Patterns".to_string(),
                library_category: LibraryCategory::FrameworkSpecific,
                category: Some("nextjs-data".to_string()),
                description: "Use appropriate Next.js data fetching methods".to_string(),
                text: "Use getServerSideProps for data that must be fresh on every request. Use getStaticProps/getStaticPaths for data that can be pre-rendered. Avoid fetching the same data multiple times.".to_string(),
                glob: Some("**/pages/**/*.{tsx,jsx}".to_string()),
                tags: vec!["nextjs".to_string(), "data-fetching".to_string()],
            },
        ]
    }

    /// Get rules by category
    pub fn by_category(category: LibraryCategory) -> Vec<Self> {
        Self::all()
            .into_iter()
            .filter(|r| r.library_category == category)
            .collect()
    }

    /// Get rules by tag
    pub fn by_tag(tag: &str) -> Vec<Self> {
        Self::all()
            .into_iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_has_rules() {
        let all = LibraryRule::all();
        assert!(!all.is_empty());
    }

    #[test]
    fn test_library_categories() {
        let security = LibraryRule::by_category(LibraryCategory::Security);
        assert!(!security.is_empty());
    }

    #[test]
    fn test_library_unique_ids() {
        let all = LibraryRule::all();
        let ids: std::collections::HashSet<_> = all.iter().map(|r| &r.id).collect();
        assert_eq!(ids.len(), all.len(), "Library rule IDs must be unique");
    }
}
