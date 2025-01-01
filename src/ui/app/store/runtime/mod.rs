pub mod editor;
pub mod generate;
pub mod review;
pub mod settings;

#[cfg(test)]
mod tests;

use super::super::LaReviewApp;
use super::command::Command;

pub fn run(app: &mut LaReviewApp, command: Command) {
    match command {
        Command::ResolveGenerateInput {
            input_text,
            selected_agent_id,
            review_id,
        } => generate::resolve_generate_input(app, input_text, selected_agent_id, review_id),
        Command::FetchPrContextPreview { input_ref } => {
            generate::fetch_pr_context_preview(app, input_ref)
        }
        Command::AbortGeneration => generate::abort_generation(app),
        Command::CheckGitHubStatus => settings::check_github_status(app),
        Command::RefreshGitHubReview {
            review_id,
            selected_agent_id,
        } => generate::refresh_github_review(app, review_id, selected_agent_id),
        Command::StartGeneration {
            run_context,
            selected_agent_id,
        } => generate::start_generation(app, *run_context, selected_agent_id),
        Command::RefreshReviewData { reason } => review::refresh_review_data(app, reason),
        Command::LoadReviewFeedbacks { review_id } => review::load_review_feedbacks(app, review_id),
        Command::LoadFeedbackLinks { review_id } => review::load_feedback_links(app, review_id),
        Command::UpdateTaskStatus { task_id, status } => {
            review::update_task_status(app, task_id, status)
        }
        Command::DeleteReview { review_id } => review::delete_review(app, review_id),
        Command::CreateFeedbackComment {
            review_id,
            task_id,
            feedback_id,
            file_path,
            line_number,
            side,
            title,
            body,
        } => review::create_feedback_comment(
            app,
            review_id,
            task_id,
            feedback_id,
            file_path,
            line_number,
            side,
            title,
            body,
        ),
        Command::RunD2 { command } => settings::run_d2_command(app, command),
        Command::GenerateExportPreview {
            review_id,
            run_id,
            include_feedback_ids,
            options,
        } => {
            review::generate_export_preview(app, review_id, run_id, include_feedback_ids, *options)
        }
        Command::ExportReview {
            review_id,
            run_id,
            path,
            options,
        } => review::export_review(app, review_id, run_id, path, *options),
        Command::UpdateFeedbackStatus {
            feedback_id,
            status,
        } => review::update_feedback_status(app, feedback_id, status),
        Command::UpdateFeedbackImpact {
            feedback_id,
            impact,
        } => review::update_feedback_impact(app, feedback_id, impact),
        Command::UpdateFeedbackTitle { feedback_id, title } => {
            review::update_feedback_title(app, feedback_id, title)
        }
        Command::SendFeedbackToPr { feedback_id } => review::send_feedback_to_pr(app, feedback_id),
        Command::SaveRepo { repo } => settings::save_repo(app, repo),
        Command::DeleteRepo { repo_id } => settings::delete_repo(app, repo_id),
        Command::PickFolderForLink => settings::pick_folder_for_link(app),
        Command::SaveAppConfigFull {
            has_seen_requirements,
            custom_agents,
            agent_path_overrides,
            agent_envs,
            preferred_editor_id,
        } => settings::save_app_config_full(
            has_seen_requirements,
            custom_agents,
            agent_path_overrides,
            agent_envs,
            preferred_editor_id,
        ),
        Command::DeleteFeedback(feedback_id) => review::delete_feedback(app, feedback_id),
        Command::DeleteComment(comment_id) => review::delete_comment(app, comment_id),
        Command::OpenInEditor {
            editor_id,
            file_path,
            line_number,
        } => editor::open_in_editor(editor_id, file_path, line_number),
    }
}
