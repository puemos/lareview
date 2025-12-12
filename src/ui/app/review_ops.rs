use crate::domain::{Note, TaskId, TaskStatus};

use super::LaReviewApp;
use super::state::AppView;

impl LaReviewApp {
    pub fn switch_to_review(&mut self) {
        self.state.current_view = AppView::Review;
        self.sync_review_from_db();
    }

    pub fn switch_to_generate(&mut self) {
        self.state.current_view = AppView::Generate;
    }

    pub fn switch_to_settings(&mut self) {
        self.state.current_view = AppView::Settings;
    }

    pub fn sync_review_from_db(&mut self) {
        match self.pr_repo.list_all() {
            Ok(prs) => {
                self.state.prs = prs;
                if self.state.selected_pr_id.is_none()
                    || !self
                        .state
                        .prs
                        .iter()
                        .any(|p| Some(&p.id) == self.state.selected_pr_id.as_ref())
                {
                    if let Some(first_pr) = self.state.prs.first() {
                        self.state.selected_pr_id = Some(first_pr.id.clone());
                    } else {
                        self.state.selected_pr_id = None;
                    }
                }
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to load pull requests: {err}"));
                return;
            }
        }

        match self.task_repo.find_all() {
            Ok(all_tasks) => {
                self.state.all_tasks = all_tasks;

                if let Some(selected_pr_id) = &self.state.selected_pr_id {
                    if let Some(pr) = self.state.prs.iter().find(|p| &p.id == selected_pr_id) {
                        self.state.pr_id = pr.id.clone();
                        self.state.pr_title = pr.title.clone();
                        self.state.pr_repo = pr.repo.clone();
                        self.state.pr_author = pr.author.clone();
                        self.state.pr_branch = pr.branch.clone();
                    }
                } else {
                    self.state.pr_id = "local-pr".to_string();
                    self.state.pr_title = "Local Review".to_string();
                    self.state.pr_repo = "local/repo".to_string();
                    self.state.pr_author = "me".to_string();
                    self.state.pr_branch = "main".to_string();
                }
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to load tasks: {err}"));
                return;
            }
        }

        let current_tasks = self.state.tasks();
        self.state.selected_task_id = self
            .state
            .selected_task_id
            .clone()
            .filter(|id| current_tasks.iter().any(|t| &t.id == id));

        if let Some(task_id) = &self.state.selected_task_id {
            if let Ok(Some(note)) = self.note_repo.find_by_task(task_id) {
                self.state.current_note = Some(note.body);
            } else {
                self.state.current_note = Some(String::new());
            }
        } else {
            self.state.current_note = None;
        }

        self.state.review_error = None;
    }

    pub fn set_task_status(&mut self, task_id: &TaskId, new_status: TaskStatus) {
        if let Err(err) = self.task_repo.update_status(task_id, new_status) {
            self.state.review_error = Some(format!("Failed to update task status: {err}"));
            return;
        }

        self.state.selected_task_id = Some(task_id.clone());
        self.state.review_error = None;
        self.sync_review_from_db();
    }

    pub fn clean_done_tasks(&mut self) {
        let Some(pr_id) = self.state.selected_pr_id.clone() else {
            return;
        };

        let done_ids = match self.task_repo.find_done_ids_by_pr(&pr_id) {
            Ok(ids) => ids,
            Err(err) => {
                self.state.review_error = Some(format!("Failed to list done tasks: {err}"));
                return;
            }
        };

        if done_ids.is_empty() {
            return;
        }

        if let Err(err) = self.note_repo.delete_by_task_ids(&done_ids) {
            self.state.review_error = Some(format!("Failed to delete notes for done tasks: {err}"));
            return;
        }

        if let Err(err) = self.task_repo.delete_by_ids(&done_ids) {
            self.state.review_error = Some(format!("Failed to delete done tasks: {err}"));
            return;
        }

        self.state.review_error = None;
        self.state.selected_task_id = None;
        self.state.current_note = None;
        self.state.current_line_note = None;
        self.sync_review_from_db();
    }

    pub fn save_current_note(&mut self) {
        let Some(task_id) = &self.state.selected_task_id else {
            return;
        };
        let body = self.state.current_note.clone().unwrap_or_default();

        let timestamp = chrono::Utc::now().to_rfc3339();
        let note = Note {
            task_id: task_id.clone(),
            body: body.clone(),
            updated_at: timestamp,
            file_path: None,
            line_number: None,
        };

        let result = self.note_repo.save(&note);

        match result {
            Ok(()) => {
                self.state.review_error = None;
            }
            Err(err) => {
                self.state.review_error = Some(format!("Failed to save note: {err}"));
            }
        }
    }
}
