# Product screenshots

The website and README share focused UI captures in `assets/screenshots/`. The hero uses the original recorded walkthrough in `assets/video/demo.mp4`, with a poster taken from that same video. They show the current workbench with an **authored, illustrative review**, not customer data or a claimed benchmark of agent performance.

## Example scenario

**Make payment webhooks safe to retry** follows a change to the fictional `platform/commerce-api` project. The sample GitLab host uses the reserved `.example` domain. The linked checkout is an example under `/Users/Shared/lareview-demo/`; it contains no personal paths or credentials.

The [sample input diff](examples/webhook-review.diff) contains six files, 32 added lines, and two removed lines. It introduces a receipt lookup, a receipt write, an unawaited payment transaction, a sequential replay test, and a 30-day retention setting.

Four review tasks cover atomic claims, acknowledgment after commit, concurrent/rollback tests, and retention documentation. The documentation task is complete; the atomic-claim task is in progress. Six issue checks contain three findings, including two open blockers:

- **Data integrity:** a receipt committed before the payment can suppress a valid retry after rollback.
- **Error handling:** the unawaited transaction allows a successful response before the payment commits.
- **Test coverage:** sequential replay does not exercise concurrent delivery or rollback followed by redelivery.

Feedback and issue checks refer to the same findings, so the summary counts two blockers once. The merge-confidence score is 2/5. The diagram focuses on the new-delivery failure path, including the unawaited branch and the receipt that remains after rollback; it does not imply that the proposed fix is already implemented. The [diagram source](examples/webhook-review.mmd) preserves the illustrated failure path. The reviewer reply in the example data describes the next action, rather than claiming the fix has shipped.

The rules use the same transaction and retry requirements. Learning patterns illustrate prior feedback preferences: prioritize observable order-state failures, skip generated-client formatting comments, and recognize explicit test fixture IDs. Their rejection counts are example history, with no pending rejections.

## Asset map

| Asset | Content |
| --- | --- |
| `generate` | Local Git composer with the same commerce-api checkout and valid main / feat/webhook-retries refs |
| `review` | Review-task component, grouped by flow with progress and risk |
| `review-analysis` | Atomic-claim verification plan and insight |
| `review-changes` | Actual handler hunk referenced by the task |
| `review-feedback-item` | Complete finding header, receipt-write anchor, and supporting explanation |
| `review-diagram` | Current new-delivery failure path |
| `rules` | Transaction rules, filtered using the real search control |
| `repos` | Linked example repository card and local-review entry point |
| `learnings` | Two enabled preferences shown in full; the example has three patterns in total |
| `export-github` | Feedback selection and explicit publish controls for the seven selected items; legacy filename retained |
| `settings` | Neutral Copilot agent row, with command details collapsed |

## Capturing and publishing

Capture the running frontend using an isolated example database and the real diff parser. Keep sample transport adapters outside production code. Navigate with the visible UI and wait for loaded content. Capture the complete feature component or a clearly bounded detail, keeping the finding, code, and explanation together. Avoid unrelated navigation, empty canvas, partial text, recording overlays, or invented controls.

Use lossless PNG captures at high display resolution, then create WebP copies at quality 92. Most current component captures use four output pixels per CSS pixel. Image dimensions follow the actual content rather than forcing every feature into a full application-window rectangle. Preserve that aspect ratio in the website and lightbox.

When an image changes, update its `?v=` URL suffix in the landing page, social-card source, and README to the first 12 characters of its SHA-256 hash. This refreshes browser and image-proxy caches without breaking existing asset paths.

Keep the existing website layout, styling, and copy when refreshing screenshots. Replace image assets and their references; a layout redesign requires an explicit request.

Keep the original hero video. Its poster must be a clean frame from the recording, rather than an unrelated app screenshot. Feature images use the current authored example; the original video remains a recorded product walkthrough.

`landing/social-card.html` is the 1200 × 630 source for `assets/og-image.jpg`. Render it from a local repository server, wait for its screenshot image to load, and capture the full card. It uses the same review-task asset, without editing the workbench pixels.

The landing production build automatically versions the stylesheet using its content hash, so a cached CSS file cannot leave a new feature layout unstyled.

After replacing assets, build `landing`, check the desktop and mobile page, and open screenshot previews with the pointer. Confirm that Escape and the close button dismiss the preview.
