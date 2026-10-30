# Product screenshots

The website and README share the assets in `assets/screenshots/`. They show the current workbench with an **authored, illustrative review**, not customer data or a claimed benchmark of agent performance.

## Example scenario

**Make payment webhooks safe to retry** follows a change to the fictional `platform/commerce-api` project. The sample GitLab host uses the reserved `.example` domain. The linked checkout is an example under `/Users/Shared/lareview-demo/`; it contains no personal paths or credentials.

The [sample input diff](examples/webhook-review.diff) contains six files, 32 added lines, and two removed lines. It introduces a receipt lookup, a receipt write, an unawaited payment transaction, a sequential replay test, and a 30-day retention setting.

Four review tasks cover atomic claims, acknowledgment after commit, concurrent/rollback tests, and retention documentation. The documentation task is complete; the atomic-claim task is in progress. Six issue checks contain three findings, including two open blockers:

- **Data integrity:** a receipt committed before the payment can suppress a valid retry after rollback.
- **Error handling:** the unawaited transaction allows a successful response before the payment commits.
- **Test coverage:** sequential replay does not exercise concurrent delivery or rollback followed by redelivery.

Feedback and issue checks refer to the same findings, so the summary counts two blockers once. The merge-confidence score is 2/5. The diagram describes the code as written, including the unawaited branch; it does not imply that the proposed fix is already implemented. The reviewer reply describes the next action, rather than claiming the fix has shipped.

The rules use the same transaction and retry requirements. Learning patterns illustrate prior feedback preferences: prioritize observable order-state failures, skip generated-client formatting comments, and recognize explicit test fixture IDs. Their rejection counts are example history, with no pending rejections.

## Asset map

| Asset | Content |
| --- | --- |
| `generate` | Copilot, linked repository, loaded merge request, actual parsed diff |
| `review` | Overview, task progress, changed-file risk, checks, feedback |
| `review-analysis` | Atomic-claim verification plan and insight |
| `review-changes` | Actual handler hunk referenced by the task |
| `review-feedback-item` | Receipt-write anchor, evidence, reviewer reply |
| `review-diagram` | Current webhook execution flow |
| `rules` | Transaction rules, filtered using the real search control |
| `repos` | Linked example repository and local-review entry point |
| `learnings` | Three enabled feedback preferences |
| `export-github` | Seven items selected for GitLab export; legacy filename retained for existing links |
| `settings` | Current agent settings, with command paths collapsed |

## Capturing and publishing

Capture the running frontend using an isolated example database and the real diff parser. Keep sample transport adapters outside production code. Navigate with the visible UI, wait for data and diagrams to finish rendering, and capture the entire app viewport. Do not paint over UI, add invented controls, or publish loading/error states. Export selection is shown before posting; no sample review is published to a remote provider.

Update both PNG and WebP for each asset. The current captures are 1430 × 914 pixels, and the WebP files use quality 92. When an image changes, update its `?v=` URL suffix in the landing page, social-card source, and README to the first 12 characters of its SHA-256 hash. This refreshes browser and image-proxy caches without breaking the existing asset paths.

Preserve the whole image on the website; the hero must not crop it or autoplay a recording of an older interface.

`landing/social-card.html` is the 1200 × 630 source for `assets/og-image.jpg`. Render it from a local repository server, wait for its screenshot image to load, and capture the full card. It uses the same overview asset, without editing the workbench pixels.

After replacing assets, build `landing`, check the desktop and mobile page, and open screenshot previews with both the keyboard and pointer. Confirm that Escape and the close button dismiss the preview and return focus to its trigger.
