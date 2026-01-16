use lareview::infra::vcs::registry::VcsRegistry;

#[test]
fn detects_github_provider_from_url() {
    let registry = VcsRegistry::default();
    let provider = registry
        .detect_provider("https://github.com/example/repo/pull/42")
        .expect("github provider");
    assert_eq!(provider.id(), "github");
}

#[test]
fn detects_gitlab_provider_from_url() {
    let registry = VcsRegistry::default();
    let provider = registry
        .detect_provider("https://gitlab.com/example/repo/-/merge_requests/7")
        .expect("gitlab provider");
    assert_eq!(provider.id(), "gitlab");
}

#[test]
fn detects_gitlab_provider_from_shorthand() {
    let registry = VcsRegistry::default();
    let provider = registry
        .detect_provider("example/repo!12")
        .expect("gitlab provider");
    assert_eq!(provider.id(), "gitlab");
}
