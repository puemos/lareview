use crate::ui::app::LaReviewApp;

impl LaReviewApp {
    pub(super) fn select_task(&mut self, task: &crate::domain::ReviewTask) {
        self.state.selected_task_id = Some(task.id.clone());
        self.state.current_line_note = None;

        if let Ok(Some(note)) = self.note_repo.find_by_task(&task.id) {
            self.state.current_note = Some(note.body);
        } else {
            self.state.current_note = Some(String::new());
        }
    }

    pub(super) fn select_task_by_id(
        &mut self,
        all_tasks: &[crate::domain::ReviewTask],
        task_id: &str,
    ) {
        if let Some(task) = all_tasks.iter().find(|t| t.id == task_id) {
            self.select_task(task);
        }
    }
}
