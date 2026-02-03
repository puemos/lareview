use crate::domain::{Comment, Feedback, MergeConfidence, Review, ReviewRun, ReviewSource, ReviewTask};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VcsStatus {
    pub id: String,
    pub name: String,
    pub cli_path: String,
    pub login: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VcsPrData {
    pub diff_text: String,
    pub title: String,
    pub source: ReviewSource,
}

#[derive(Debug, Clone)]
pub struct VcsCloneRequest {
    pub repo: String,
    pub dest_path: PathBuf,
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VcsCloneResult {
    pub path: PathBuf,
}

pub trait VcsRef: Send + Sync {
    fn provider_id(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone)]
pub struct ReviewPushRequest {
    pub review: Review,
    pub run: ReviewRun,
    pub tasks: Vec<ReviewTask>,
    pub feedbacks: Vec<Feedback>,
    pub comments: Vec<Comment>,
    pub selected_tasks: Vec<String>,
    pub selected_feedbacks: Vec<String>,
    pub merge_confidence: Option<MergeConfidence>,
}

#[derive(Debug, Clone)]
pub struct FeedbackPushRequest {
    pub review: Review,
    pub run: ReviewRun,
    pub feedback: Feedback,
    pub comments: Vec<Comment>,
}

#[async_trait]
pub trait VcsProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn matches_ref(&self, reference: &str) -> bool;
    fn parse_ref(&self, reference: &str) -> Option<Box<dyn VcsRef>>;
    async fn fetch_pr(&self, reference: &dyn VcsRef) -> Result<VcsPrData>;
    async fn push_review(&self, request: ReviewPushRequest) -> Result<String>;
    async fn push_feedback(&self, request: FeedbackPushRequest) -> Result<String>;
    async fn clone_repo(&self, request: VcsCloneRequest) -> Result<VcsCloneResult>;
    async fn get_status(&self) -> Result<VcsStatus>;
}
