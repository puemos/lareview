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
        Command::LoadReviewThreads { review_id } => review::load_review_threads(app, review_id),
        Command::UpdateTaskStatus { task_id, status } => {
            review::update_task_status(app, task_id, status)
        }
        Command::DeleteReview { review_id } => review::delete_review(app, review_id),
        Command::CreateThreadComment {
            review_id,
            task_id,
            thread_id,
            file_path,
            line_number,
            title,
            body,
        } => review::create_thread_comment(
            app,
            review_id,
            task_id,
            thread_id,
            file_path,
            line_number,
            title,
            body,
        ),
        Command::RunD2 { command } => settings::run_d2_command(app, command),
        Command::GenerateExportPreview { review_id, run_id } => {
            review::generate_export_preview(app, review_id, run_id)
        }
        Command::ExportReview {
            review_id,
            run_id,
            path,
        } => review::export_review(app, review_id, run_id, path),
        Command::UpdateThreadStatus { thread_id, status } => {
            review::update_thread_status(app, thread_id, status)
        }
        Command::UpdateThreadImpact { thread_id, impact } => {
            review::update_thread_impact(app, thread_id, impact)
        }
        Command::UpdateThreadTitle { thread_id, title } => {
            review::update_thread_title(app, thread_id, title)
        }
        Command::SaveRepo { repo } => settings::save_repo(app, repo),
        Command::DeleteRepo { repo_id } => settings::delete_repo(app, repo_id),
        Command::PickFolderForLink => settings::pick_folder_for_link(app),
        Command::SaveAppConfig {
            extra_path,
            has_seen_requirements,
        } => settings::save_app_config_full(
            extra_path,
            has_seen_requirements,
            Vec::new(),
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
        ),
        Command::SaveAppConfigFull {
            extra_path,
            has_seen_requirements,
            custom_agents,
            agent_path_overrides,
            agent_envs,
        } => settings::save_app_config_full(
            extra_path,
            has_seen_requirements,
            custom_agents,
            agent_path_overrides,
            agent_envs,
        ),
    }
}
