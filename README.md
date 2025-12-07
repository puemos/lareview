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
