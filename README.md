# Code Review is Stuck. Let's Reimagine It.

For years, the way we review code has been stuck. We've been using the same basic tool: a simple, file-by-file diff with a thread-like system for async conversations. It was a fine start, but it hasn't fundamentally changed.

Lately, some attempts to build upon this structure have been made. Tools have added suggestions, better integrations, and more notifications. But these are just incremental improvements. Instead of reimagining the developer experience, they've just added more layers on top of a dated foundation.

The core problem remains: a flat list of file changes doesn't represent how we think about code. A feature isn't just a collection of random edits, it's a logical flow with a clear purpose.

## LaReview: A New DX for Code Review

LaReview is a fundamental shift in the code review experience. It's built on the idea that a review shouldn't be a passive scroll through a diff, but an active process of verifying a change's intent.

We do this by structuring every review around **Intents** and **Sub-flows**:

1.  **Intent:** This is the high-level goal. What is this PR *really* trying to achieve? (e.g., "Implement password reset flow").
2.  **Sub-flows:** These are the concrete, verifiable steps required to make the intent a reality. (e.g., "Generate unique token," "Send reset email," "Validate token," "Allow password update").

This approach creates a logical checklist for every PR, making reviews more thorough and less overwhelming. It helps you focus on the architectural soundness of a change, not just the syntax of individual lines. You're no longer just reviewing code; you're validating a story.

For this structured approach, LaReview employs an AI agent. This agent communicates via the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/overview/introduction), analyzing the provided diff to quickly generate the Intent and Sub-flow review plan. This automates change decomposition, letting developers focus on critical review aspects and maintain high standards efficiently.

## A New UI for a New DX

To complement this new approach, LaReview introduces a new user interface with two main views: `GENERATE` and `REVIEW`.

### GENERATE View

The `GENERATE` view is where you initiate the review process. You can paste a diff, select an AI agent, and generate a review plan. This view is designed to be simple and intuitive, allowing you to quickly get started with your review.

### REVIEW View

The `REVIEW` view is where you'll spend most of your time. It features a new tree-based navigation system that allows you to easily navigate through the review plan. The view is divided into two panels: a navigation tree on the left and a content view on the right.

The navigation tree displays a hierarchical view of the review plan, with the Intent at the top and the Sub-flows and Tasks nested below. This allows you to easily see the structure of the review and jump to any part of it.

The content view displays the details of the selected item in the navigation tree. This can be the Intent, a Sub-flow, or a Task. For a Task, you'll see the description, AI insights, and a unified diff viewer that shows all the changes related to that task.

Tasks can be marked as **To Do**, **In Progress**, **Done**, or **Ignored** from the task detail pane. The progress bar and sub-flow counts reflect completion. Use **Clean done** in the Review header to remove completed tasks (and their notes) for the current PR.

### Data Model

LaReview's new data model is designed to support this new workflow. It includes three main entities:

*   **PullRequest:** Represents a pull request and contains information such as the title, description, and author.
*   **ReviewTask:** Represents a single task in the review plan. It includes a description, AI insights, and a list of diffs.
*   **Note:** Allows you to add your own notes to a task.

This new data model, combined with the new UI and the powerful AI agent, makes LaReview a truly unique and powerful tool for code review.
