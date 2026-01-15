use crate::infra::vcs::traits::VcsProvider;
use crate::infra::vcs::{github::GitHubProvider, gitlab::GitLabProvider};

pub struct VcsRegistry {
    providers: Vec<Box<dyn VcsProvider>>,
}

impl Default for VcsRegistry {
    fn default() -> Self {
        Self {
            providers: vec![
                Box::new(GitHubProvider::new()),
                Box::new(GitLabProvider::new()),
            ],
        }
    }
}

impl VcsRegistry {
    pub fn detect_provider(&self, reference: &str) -> Option<&dyn VcsProvider> {
        self.providers
            .iter()
            .map(|provider| provider.as_ref())
            .find(|provider| provider.matches_ref(reference))
    }

    pub fn get_provider(&self, id: &str) -> Option<&dyn VcsProvider> {
        self.providers
            .iter()
            .map(|provider| provider.as_ref())
            .find(|provider| provider.id() == id)
    }

    pub fn providers(&self) -> Vec<&dyn VcsProvider> {
        self.providers
            .iter()
            .map(|provider| provider.as_ref())
            .collect()
    }
}
