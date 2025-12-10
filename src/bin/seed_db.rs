use rusqlite::Connection;
use std::fs;

// Simple structs matching the database schema
#[derive(Debug)]
struct PullRequest {
    id: String,
    title: String,
    description: Option<String>,
    repo: String,
    author: String,
    branch: String,
    created_at: String,
}

#[derive(Debug)]
struct ReviewTask {
    id: String,
    pr_id: String,
    title: String,
    description: String,
    files: String, // JSON string
    stats: String, // JSON string
    insight: Option<String>,
    patches: Option<String>, // JSON string
    diagram: Option<String>,
    ai_generated: bool,
    status: String,
    sub_flow: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Determine database path
    let db_path = if let Ok(path) = std::env::var("LAREVIEW_DB_PATH") {
        std::path::PathBuf::from(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        cwd.join(".lareview").join("db.sqlite")
    };

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    println!("Connecting to database at: {}", db_path.display());

    let conn = Connection::open(&db_path)?;

    // Sample PR based on cal.com booking audit and booking logs changes
    let pr = PullRequest {
        id: "pr-9821-booking-audit-timeline".to_string(),
        title: "Improve booking audit timeline and action registry".to_string(),
        repo: "calcom/cal.com".to_string(),
        author: "contributor".to_string(),
        branch: "feat/booking-audit-timeline".to_string(),
        created_at: "2024-11-15T14:32:00Z".to_string(),
        description: Some(
            "## Summary\n\
This PR improves the booking audit subsystem and the booking logs UI.\n\
It introduces a centralized registry for audit action services, typed action data,\n\
timezone-aware display helpers, and a richer booking logs timeline that surfaces\n\
avatars, roles, and JSON details in a more readable way.\n\
\n\
## Frontend\n\
- Booking logs view renders translated action titles with support for embedded\n\
  components (for example, links to related bookings).\n\
- Timeline rows show actor avatar, display name, and a simple role label\n\
  for guest, attendee, user, and system.\n\
- Details drawer is simplified to show typed display fields and an opt-in\n\
  JSON viewer with line numbers.\n\
- Type filter is removed in favor of a simpler actor filter and free text search.\n\
\n\
## Backend\n\
- Introduces BookingAuditActionServiceRegistry as the central mapping\n\
  from booking audit action to its action service.\n\
- Expands all audit action services to provide:\n\
  - migrateToLatest for versioned payloads\n\
  - getDisplayTitle with translation key and params\n\
  - optional getDisplayJson and getDisplayFields hooks for the viewer.\n\
- Adds BookingStatusChangeSchema and other typed zod schemas for action data.\n\
- Refactors BookingAuditTaskConsumer to use a lean base task schema and\n\
  delegate validation and migration to the appropriate action service.\n\
- Refactors BookingAuditTaskerProducerService to expose strongly typed\n\
  queue methods for each action type, while keeping a legacy queueAudit\n\
  entry point.\n\
- BookingAuditViewerService now enriches audit logs with actionDisplayTitle,\n\
  typed display data, and optional displayFields for the UI.\n\
- For bookings created from a reschedule, the viewer also pulls the last\n\
  RESCHEDULED log from the previous booking and renders a synthetic\n\
  'rescheduled from' entry at the top of the timeline.\n\
\n\
## Data and repositories\n\
- No schema changes.\n\
- Booking repository exposes getFromRescheduleUid to identify bookings that\n\
  originated from a reschedule.\n\
- Booking audit repository exposes findRescheduledLogsOfBooking to support\n\
  the 'rescheduled from' view.\n\
\n\
## Tasker and DI\n\
- Tasker bookingAudit payload is now BookingAuditTaskBasePayload, with\n\
  action specific validation done in the consumer.\n\
- New DI modules wire BookingAuditViewerService with the booking audit\n\
  repository and booking repository.\n\
\n\
## Risks\n\
- Misconfigured action registry entries will surface as runtime errors when\n\
  queueing or consuming audit tasks.\n\
- Incorrect timezone handling would show confusing timestamps in the\n\
  booking logs timeline.\n\
\n\
## Testing\n\
- Verified booking logs timeline for bookings with created, rescheduled,\n\
  accepted, cancelled, and reassignment events.\n\
- Verified 'rescheduled from' shows up for bookings created from a\n\
  reschedule.\n\
- Manually exercised JSON viewer toggle and ensured it handles empty\n\
  payloads gracefully.\n"
                .to_string(),
        ),
    };

    conn.execute(
        "INSERT OR REPLACE INTO pull_requests (id, title, description, repo, author, branch, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&pr.id, &pr.title, &pr.description, &pr.repo, &pr.author, &pr.branch, &pr.created_at),
    )?;
    println!("Inserted PR: {}", pr.title);

    // Sample tasks modeled after real review items from the cal.com diff
    let tasks = vec![
        // Booking logs UI and i18n
        ReviewTask {
            id: "booking-logs-ui-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review booking logs timeline UI and i18n wiring".to_string(),
            description: "Review the changes in the booking logs timeline UI. Confirm that the new ActionTitle and JsonViewer components integrate correctly with the tRPC response shape, that avatars and actor roles are rendered safely, and that we do not regress accessibility or i18n behavior. Pay attention to how booking_audit_action.* keys are used and how link components are passed through ServerTrans.".to_string(),
            files: r#"[
  "apps/web/modules/booking/logs/views/booking-logs-view.tsx",
  "apps/web/public/static/locales/en/common.json"
]"#.to_string(),
            stats: r#"{"additions": 210, "deletions": 75, "risk": "MEDIUM", "tags": ["booking-logs", "react", "i18n", "ui"]}"#.to_string(),
            insight: Some("This UI is a primary debugging surface for support and customers. A small regression in how we format dates, JSON, or links will be very visible. Treat the JsonViewer as a mini log viewer and think about default collapsed behavior and performance on large payloads.".to_string()),
            patches: Some(
                r#"[
  {
    "file": "apps/web/modules/booking/logs/views/booking-logs-view.tsx",
    "hunk": "diff --git a/apps/web/modules/booking/logs/views/booking-logs-view.tsx b/apps/web/modules/booking/logs/views/booking-logs-view.tsx\n--- a/apps/web/modules/booking/logs/views/booking-logs-view.tsx\n+++ b/apps/web/modules/booking/logs/views/booking-logs-view.tsx\n@@ -3,6 +3,7 @@\n \"use client\";\n \n+import Link from \"next/link\";\n import { useRouter } from \"next/navigation\";\n import { useState } from \"react\";\n import { useLocale } from \"@calcom/lib/hooks/useLocale\";\n@@ -12,6 +13,8 @@\n import { Button } from \"@calcom/ui/components/button\";\n import { Icon } from \"@calcom/ui/components/icon\";\n import { SkeletonText } from \"@calcom/ui/components/skeleton\";\n import { FilterSearchField, Select } from \"@calcom/ui/components/form\";\n+import { Avatar } from \"@calcom/ui/components/avatar\";\n+import ServerTrans from \"@calcom/lib/components/ServerTrans\";\n@@\n type AuditLog = {\n     id: string;\n     action: string;\n     type: string;\n     timestamp: string;\n     data: Record<string, unknown> | null;\n+    actionDisplayTitle: TranslationWithParams;\n+    displayFields?: Array<{ labelKey: string; valueKey: string }>;\n     actor: {\n         type: string;\n         displayName: string | null;\n+        displayEmail: string | null;\n+        displayAvatar: string | null;\n     };\n };\n@@\n+function ActionTitle({ actionDisplayTitle }: { actionDisplayTitle: TranslationWithParams }) {\n+    const { t } = useLocale();\n+\n+    if (actionDisplayTitle.components?.length) {\n+        return (\n+            <ServerTrans\n+                t={t}\n+                i18nKey={actionDisplayTitle.key}\n+                values={actionDisplayTitle.params}\n+                components={actionDisplayTitle.components.map((comp) =>\n+                    comp.type === \"link\" ? (\n+                        <Link\n+                            key={comp.href}\n+                            href={comp.href}\n+                            className=\"text-emphasis underline hover:no-underline\"\n+                        />\n+                    ) : (\n+                        <span key={comp.href} />\n+                    )\n+                )}\n+            />\n+        );\n+    }\n+\n+    return <>{t(actionDisplayTitle.key, actionDisplayTitle.params)}</>;\n+}\n"
  },
  {
    "file": "apps/web/public/static/locales/en/common.json",
    "hunk": "diff --git a/apps/web/public/static/locales/en/common.json b/apps/web/public/static/locales/en/common.json\n--- a/apps/web/public/static/locales/en/common.json\n+++ b/apps/web/public/static/locales/en/common.json\n@@ -4095,8 +4095,25 @@\n-  \"audit_action\": {\n-    \"created\": \"Created\"\n+  \"booking_audit_action\": {\n+    \"created\": \"Created\",\n+    \"cancelled\": \"Cancelled\",\n+    \"rescheduled\": \"Rescheduled {{oldDate}} -> <0>{{newDate}}</0>\",\n+    \"rescheduled_from\": \"Rescheduled <0>{{oldDate}}</0> -> {{newDate}}\",\n+    \"accepted\": \"Accepted\",\n+    \"reschedule_requested\": \"Reschedule Requested\",\n+    \"attendee_added\": \"Attendee Added\",\n+    \"host_no_show_updated\": \"Host No-Show Updated\",\n+    \"rejected\": \"Rejected\",\n+    \"attendee_removed\": \"Attendee Removed\",\n+    \"reassignment\": \"Reassignment\",\n+    \"booking_reassigned_to_host\": \"Booking reassigned to {{host}}\",\n+    \"location_changed\": \"Location Changed\",\n+    \"location_changed_from_to\": \"Location changed from {{fromLocation}} to {{toLocation}}\",\n+    \"attendee_no_show_updated\": \"Attendee No-Show Updated\",\n+    \"type\": \"Assignment Type\",\n+    \"assignmentType_manual\": \"Manual Assignment\",\n+    \"assignmentType_roundRobin\": \"Round Robin Assignment\"\n   },\n"
  }
]"#.trim().to_string(),
            ),
            diagram: Some(
                r#"direction: right

Client: {
  shape: person
  label: User
}

WebApp: {
  label: "Next.js\nBookingLogsView"
}

API: {
  label: "tRPC\ngetAuditLogs"
}

ViewerService: {
  label: BookingAuditViewerService
}

Repos: {
  shape: cylinder
  label: Repositories
}

Client -> WebApp: Open booking logs
WebApp -> API: getAuditLogs
API -> ViewerService: Fetch audit data
ViewerService -> Repos: Load logs & metadata
Repos -> ViewerService: Raw audit records
ViewerService -> API: Enriched logs
API -> WebApp: JSON response
WebApp -> Client: Render timeline"#.trim().to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Booking logs UI".to_string()),
        },

        // Action services, schemas, and registry
        ReviewTask {
            id: "booking-audit-actions-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review booking audit action services and registry".to_string(),
            description: "Review the BookingAuditActionServiceRegistry and the action services under packages/features/booking-audit/lib/actions. Confirm that all BookingAuditAction variants are wired into the registry, that migrateToLatest, getDisplayTitle, getDisplayJson, and getDisplayFields contracts are consistent with IAuditActionService, and that BookingStatusChangeSchema is used where appropriate. Pay attention to how translation keys and params are produced for frontend use.".to_string(),
            files: r#"[
  "packages/features/booking-audit/lib/actions/IAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/CreatedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/RescheduledAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/ReassignmentAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/AcceptedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/CancelledAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/RescheduleRequestedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/AttendeeAddedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/AttendeeRemovedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/HostNoShowUpdatedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/AttendeeNoShowUpdatedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/LocationChangedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/RejectedAuditActionService.ts",
  "packages/features/booking-audit/lib/common/changeSchemas.ts",
  "packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts"
]"#.to_string(),
            stats: r#"{"additions": 260, "deletions": 40, "risk": "HIGH", "tags": ["booking-audit", "typescript", "domain-model", "i18n"]}"#.to_string(),
            insight: Some("These action services define the contract between producers, consumers, and the booking logs UI. If the registry mapping or zod schemas drift, we will either drop logs or misrender titles. Review the registry as a single source of truth and think about how we would add a new action in the future without touching too many files.".to_string()),
            patches: Some(
                r#"[
  {
    "file": "packages/features/booking-audit/lib/actions/IAuditActionService.ts",
    "hunk": "diff --git a/packages/features/booking-audit/lib/actions/IAuditActionService.ts b/packages/features/booking-audit/lib/actions/IAuditActionService.ts\n--- a/packages/features/booking-audit/lib/actions/IAuditActionService.ts\n+++ b/packages/features/booking-audit/lib/actions/IAuditActionService.ts\n@@ -64,11 +64,34 @@\n-    getDisplayJson?(storedData: { version: number; fields: z.infer<TStoredFieldsSchema> }): unknown;\n+    getDisplayJson?(params: { storedData: { version: number; fields: z.infer<TStoredFieldsSchema> }; userTimeZone: string }): unknown;\n@@\n-    getDisplayTitle(storedData: { version: number; fields: z.infer<TStoredFieldsSchema> }): Promise<TranslationWithParams>;\n+    getDisplayTitle(params: { storedData: { version: number; fields: z.infer<TStoredFieldsSchema> }; userTimeZone: string }): Promise<TranslationWithParams>;\n+\n+    getDisplayFields?(storedData: { version: number; fields: z.infer<TStoredFieldsSchema> }): Array<{\n+        labelKey: string;\n+        valueKey: string;\n+    }>;\n"
  },
  {
    "file": "packages/features/booking-audit/lib/common/changeSchemas.ts",
    "hunk": "diff --git a/packages/features/booking-audit/lib/common/changeSchemas.ts b/packages/features/booking-audit/lib/common/changeSchemas.ts\n--- a/packages/features/booking-audit/lib/common/changeSchemas.ts\n+++ b/packages/features/booking-audit/lib/common/changeSchemas.ts\n@@ -1,5 +1,6 @@\n import { z } from \"zod\";\n \n+import { BookingStatus } from \"@calcom/prisma/enums\";\n@@\n+export const BookingStatusChangeSchema = z.object({\n+    old: z.nativeEnum(BookingStatus).nullable(),\n+    new: z.nativeEnum(BookingStatus),\n+});\n"
  },
  {
    "file": "packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts",
    "hunk": "diff --git a/packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts b/packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts\nnew file mode 100644\n--- /dev/null\n+++ b/packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts\n@@ -0,0 +1,87 @@\n+import type { UserRepository } from \"@calcom/features/users/repositories/UserRepository\";\n+import type { IAuditActionService } from \"../actions/IAuditActionService\";\n+import type { BookingAuditAction } from \"../repository/IBookingAuditRepository\";\n+\n+import { CreatedAuditActionService, type CreatedAuditData } from \"../actions/CreatedAuditActionService\";\n+import { CancelledAuditActionService, type CancelledAuditData } from \"../actions/CancelledAuditActionService\";\n+import { RescheduledAuditActionService, type RescheduledAuditData } from \"../actions/RescheduledAuditActionService\";\n+import { AcceptedAuditActionService, type AcceptedAuditData } from \"../actions/AcceptedAuditActionService\";\n+import { RescheduleRequestedAuditActionService, type RescheduleRequestedAuditData } from \"../actions/RescheduleRequestedAuditActionService\";\n+import { AttendeeAddedAuditActionService, type AttendeeAddedAuditData } from \"../actions/AttendeeAddedAuditActionService\";\n+import { HostNoShowUpdatedAuditActionService, type HostNoShowUpdatedAuditData } from \"../actions/HostNoShowUpdatedAuditActionService\";\n+import { RejectedAuditActionService, type RejectedAuditData } from \"../actions/RejectedAuditActionService\";\n+import { AttendeeRemovedAuditActionService, type AttendeeRemovedAuditData } from \"../actions/AttendeeRemovedAuditActionService\";\n+import { ReassignmentAuditActionService, type ReassignmentAuditData } from \"../actions/ReassignmentAuditActionService\";\n+import { LocationChangedAuditActionService, type LocationChangedAuditData } from \"../actions/LocationChangedAuditActionService\";\n+import { AttendeeNoShowUpdatedAuditActionService, type AttendeeNoShowUpdatedAuditData } from \"../actions/AttendeeNoShowUpdatedAuditActionService\";\n+\n+export type AuditActionData =\n+    | CreatedAuditData\n+    | CancelledAuditData\n+    | RescheduledAuditData\n+    | AcceptedAuditData\n+    | RescheduleRequestedAuditData\n+    | AttendeeAddedAuditData\n+    | HostNoShowUpdatedAuditData\n+    | RejectedAuditData\n+    | AttendeeRemovedAuditData\n+    | ReassignmentAuditData\n+    | LocationChangedAuditData\n+    | AttendeeNoShowUpdatedAuditData;\n+\n+interface BookingAuditActionServiceRegistryDeps {\n+    userRepository: UserRepository;\n+}\n+\n+export class BookingAuditActionServiceRegistry {\n+    private readonly actionServices: Map<BookingAuditAction, IAuditActionService<any, any>>;\n+\n+    constructor(private deps: BookingAuditActionServiceRegistryDeps) {\n+        const services: Array<[BookingAuditAction, IAuditActionService<any, any>]> = [\n+            [\"CREATED\", new CreatedAuditActionService()],\n+            [\"CANCELLED\", new CancelledAuditActionService()],\n+            [\"RESCHEDULED\", new RescheduledAuditActionService()],\n+            [\"ACCEPTED\", new AcceptedAuditActionService()],\n+            [\"RESCHEDULE_REQUESTED\", new RescheduleRequestedAuditActionService()],\n+            [\"ATTENDEE_ADDED\", new AttendeeAddedAuditActionService()],\n+            [\"HOST_NO_SHOW_UPDATED\", new HostNoShowUpdatedAuditActionService()],\n+            [\"REJECTED\", new RejectedAuditActionService()],\n+            [\"ATTENDEE_REMOVED\", new AttendeeRemovedAuditActionService()],\n+            [\"REASSIGNMENT\", new ReassignmentAuditActionService(deps.userRepository)],\n+            [\"LOCATION_CHANGED\", new LocationChangedAuditActionService()],\n+            [\"ATTENDEE_NO_SHOW_UPDATED\", new AttendeeNoShowUpdatedAuditActionService()],\n+        ];\n+        this.actionServices = new Map(services);\n+    }\n+\n+    getActionService(action: BookingAuditAction): IAuditActionService<any, any> {\n+        const service = this.actionServices.get(action);\n+        if (!service) {\n+            throw new Error(`No action service found for: ${action}`);\n+        }\n+        return service;\n+    }\n+}\n"
  }
]"#.trim().to_string(),
            ),
            diagram: Some(
                r#"direction: right

Events: {
  shape: oval
  label: "Booking Events"
}

Producer: {
  label: "Producer Service"
}

Tasker: {
  shape: queue
  label: "Tasker Queue"
}

Consumer: {
  label: "Task Consumer"
}

Registry: {
  shape: hexagon
  label: "Action Registry"
}

ActionServices: {
  label: "Action Services\nCreated, Rescheduled\nCancelled, ..."
}

Repo: {
  shape: cylinder
  label: "Audit Repository"
}

Events -> Producer: Queue audit
Producer -> Tasker: Enqueue task
Tasker -> Consumer: Deliver payload
Consumer -> Registry: Get service
Registry -> ActionServices: Route to action
ActionServices -> Consumer: Process & migrate
Consumer -> Repo: Store versioned data"#.trim().to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Action services and registry".to_string()),
        },

        // Task consumer, producer, and task payload
        ReviewTask {
            id: "booking-audit-tasks-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review booking audit task consumer and producer pipeline".to_string(),
            description: "Review the changes in BookingAuditTaskConsumer, BookingAuditTaskerProducerService, bookingAuditTask types, and tasker.ts. Confirm that the lean BookingAuditTaskBasePayload is sufficient for routing, that action-specific zod validation occurs in the right place, and that legacy queueAudit usage remains safe. Pay attention to how errors are logged, how organizationId null cases are handled, and how IS_PRODUCTION influences queueing behavior.".to_string(),
            files: r#"[
  "packages/features/booking-audit/lib/service/BookingAuditTaskConsumer.ts",
  "packages/features/booking-audit/lib/service/BookingAuditTaskerProducerService.ts",
  "packages/features/booking-audit/lib/service/BookingAuditProducerService.interface.ts",
  "packages/features/booking-audit/lib/types/bookingAuditTask.ts",
  "packages/features/tasker/tasker.ts"
]"#.to_string(),
            stats: r#"{"additions": 310, "deletions": 120, "risk": "HIGH", "tags": ["tasker", "queue", "booking-audit", "typescript"]}"#.to_string(),
            insight: Some("This is a good place to think about failure modes. What happens if an action is added to the enum but not to the registry, or vice versa. What happens if a producer passes the wrong data shape. Consider logging, observability, and how we might backfill or replay audit tasks if something goes wrong.".to_string()),
            patches: Some(
                r#"[
  {
    "file": "packages/features/booking-audit/lib/types/bookingAuditTask.ts",
    "hunk": "diff --git a/packages/features/booking-audit/lib/types/bookingAuditTask.ts b/packages/features/booking-audit/lib/types/bookingAuditTask.ts\n--- a/packages/features/booking-audit/lib/types/bookingAuditTask.ts\n+++ b/packages/features/booking-audit/lib/types/bookingAuditTask.ts\n@@ -1,29 +1,72 @@\n import { z } from \"zod\";\n@@\n-import {\n-    CreatedAuditActionService,\n-} from \"../actions/CreatedAuditActionService\";\n+import { CreatedAuditActionService } from \"../actions/CreatedAuditActionService\";\n+import { RescheduledAuditActionService } from \"../actions/RescheduledAuditActionService\";\n+import { AcceptedAuditActionService } from \"../actions/AcceptedAuditActionService\";\n+import { CancelledAuditActionService } from \"../actions/CancelledAuditActionService\";\n+import { RescheduleRequestedAuditActionService } from \"../actions/RescheduleRequestedAuditActionService\";\n+import { AttendeeAddedAuditActionService } from \"../actions/AttendeeAddedAuditActionService\";\n+import { HostNoShowUpdatedAuditActionService } from \"../actions/HostNoShowUpdatedAuditActionService\";\n+import { RejectedAuditActionService } from \"../actions/RejectedAuditActionService\";\n+import { AttendeeRemovedAuditActionService } from \"../actions/AttendeeRemovedAuditActionService\";\n+import { ReassignmentAuditActionService } from \"../actions/ReassignmentAuditActionService\";\n+import { LocationChangedAuditActionService } from \"../actions/LocationChangedAuditActionService\";\n+import { AttendeeNoShowUpdatedAuditActionService } from \"../actions/AttendeeNoShowUpdatedAuditActionService\";\n+\n+export const BookingAuditActionSchema = z.enum([\n+    \"CREATED\",\n+    \"RESCHEDULED\",\n+    \"ACCEPTED\",\n+    \"CANCELLED\",\n+    \"RESCHEDULE_REQUESTED\",\n+    \"ATTENDEE_ADDED\",\n+    \"HOST_NO_SHOW_UPDATED\",\n+    \"REJECTED\",\n+    \"ATTENDEE_REMOVED\",\n+    \"REASSIGNMENT\",\n+    \"LOCATION_CHANGED\",\n+    \"ATTENDEE_NO_SHOW_UPDATED\",\n+]);\n+\n+export type BookingAuditAction = z.infer<typeof BookingAuditActionSchema>;\n@@\n-export const BookingAuditTaskConsumerPayloadSchema = z.discriminatedUnion(\"action\", [\n-    baseSchema.merge(z.object({\n-        action: z.literal(CreatedAuditActionService.TYPE),\n-        data: CreatedAuditActionService.storedFieldsSchema,\n-    })),\n-]);\n-\n-export type BookingAuditTaskConsumerPayload = z.infer<typeof BookingAuditTaskConsumerPayloadSchema>;\n+export const BookingAuditTaskBaseSchema = z.object({\n+    bookingUid: z.string(),\n+    actor: ActorSchema,\n+    organizationId: z.number().nullable(),\n+    timestamp: z.number(),\n+    action: BookingAuditActionSchema,\n+    data: z.unknown(),\n+});\n+\n+export type BookingAuditTaskBasePayload = z.infer<typeof BookingAuditTaskBaseSchema>;\n"
  },
  {
    "file": "packages/features/tasker/tasker.ts",
    "hunk": "diff --git a/packages/features/tasker/tasker.ts b/packages/features/tasker/tasker.ts\n--- a/packages/features/tasker/tasker.ts\n+++ b/packages/features/tasker/tasker.ts\n@@ -1,7 +1,7 @@\n import type { z } from \"zod\";\n@@\n-import type { BookingAuditTaskConsumerPayload } from \"@calcom/features/booking-audit/lib/types/bookingAuditTask\";\n+import type { BookingAuditTaskBasePayload } from \"@calcom/features/booking-audit/lib/types/bookingAuditTask\";\n@@\n-  bookingAudit: BookingAuditTaskConsumerPayload;\n+  bookingAudit: BookingAuditTaskBasePayload;\n"
  }
]"#.trim().to_string(),
            ),
            diagram: Some(
                r#"direction: right

API: {
  label: "Booking Service"
}

Producer: {
  label: "BookingAuditTasker\nProducerService"
}

Tasker: {
  label: "Tasker\nbookingAudit"
}

Consumer: {
  label: "BookingAuditTask\nConsumer"
}

Registry: {
  shape: hexagon
  label: "Action Service\nRegistry"
}

Repo: {
  shape: cylinder
  label: "BookingAudit\nRepository"
}

API -> Producer: "queueCreatedAudit\nqueueRescheduledAudit"
Producer -> Tasker: "create task\nwith base payload"
Tasker -> Consumer: deliver payload
Consumer -> Registry: getActionService
Registry -> Consumer: return typed service
Consumer -> Repo: "insert audit row\nwith versioned data""#.trim().to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Task processing pipeline".to_string()),
        },

        // Viewer service and reschedule context
        ReviewTask {
            id: "booking-audit-viewer-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review BookingAuditViewerService and reschedule context handling".to_string(),
            description: "Review BookingAuditViewerService and the new container module. Confirm that getAuditLogsForBooking enriches actors correctly, calls getDisplayTitle and getDisplayJson on the right action services, and handles missing or malformed data defensively. Pay special attention to the rescheduled from logic that pulls RESCHEDULED logs from the previous booking and injects a synthetic entry at the top of the timeline for the current booking.".to_string(),
            files: r#"[
  "packages/features/booking-audit/lib/service/BookingAuditViewerService.ts",
  "packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts",
  "packages/features/booking-audit/lib/repository/IBookingAuditRepository.ts",
  "packages/features/booking-audit/lib/repository/PrismaBookingAuditRepository.ts",
  "packages/features/bookings/repositories/BookingRepository.ts",
  "packages/features/di/containers/BookingAuditViewerService.container.ts"
]"#.to_string(),
            stats: r#"{"additions": 230, "deletions": 60, "risk": "MEDIUM", "tags": ["booking-audit", "viewer", "typescript"]}"#.to_string(),
            insight: Some("The viewer service is the bridge between storage and UI. The new 'rescheduled from' synthetic log is a good place to look for off-by-one style bugs or confusing ownership of bookingUid. Think about how this behaves for long reschedule chains and how we might test that in isolation.".to_string()),
            patches: Some(
                r#"[
  {
    "file": "packages/features/booking-audit/lib/service/BookingAuditViewerService.ts",
    "hunk": "diff --git a/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts b/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts\n--- a/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts\n+++ b/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts\n@@ -1,14 +1,16 @@\n import type { UserRepository } from \"@calcom/features/users/repositories/UserRepository\";\n+import type { BookingRepository } from \"@calcom/features/bookings/repositories/BookingRepository\";\n \n-import { CreatedAuditActionService, type CreatedAuditDisplayData } from \"../actions/CreatedAuditActionService\";\n+import { BookingAuditActionServiceRegistry } from \"./BookingAuditActionServiceRegistry\";\n+import type { TranslationWithParams } from \"../actions/IAuditActionService\";\n+import { RescheduledAuditActionService } from \"../actions/RescheduledAuditActionService\";\n@@\n-interface BookingAuditViewerServiceDeps {\n-    bookingAuditRepository: IBookingAuditRepository;\n-    userRepository: UserRepository;\n-}\n+interface BookingAuditViewerServiceDeps {\n+    bookingAuditRepository: IBookingAuditRepository;\n+    userRepository: UserRepository;\n+    bookingRepository: BookingRepository;\n+}\n@@\n export class BookingAuditViewerService {\n-    private readonly createdActionService: CreatedAuditActionService;\n+    private readonly actionServiceRegistry: BookingAuditActionServiceRegistry;\n@@\n-    constructor(private readonly deps: BookingAuditViewerServiceDeps) {\n-        this.bookingAuditRepository = deps.bookingAuditRepository;\n-        this.userRepository = deps.userRepository;\n-\n-        this.createdActionService = new CreatedAuditActionService();\n-    }\n+    constructor(private readonly deps: BookingAuditViewerServiceDeps) {\n+        this.bookingAuditRepository = deps.bookingAuditRepository;\n+        this.userRepository = deps.userRepository;\n+        this.bookingRepository = deps.bookingRepository;\n+        this.rescheduledAuditActionService = new RescheduledAuditActionService();\n+\n+        this.actionServiceRegistry = new BookingAuditActionServiceRegistry({ userRepository: this.userRepository });\n+    }\n"
  },
  {
    "file": "packages/features/booking-audit/lib/repository/IBookingAuditRepository.ts",
    "hunk": "diff --git a/packages/features/booking-audit/lib/repository/IBookingAuditRepository.ts b/packages/features/booking-audit/lib/repository/IBookingAuditRepository.ts\n--- a/packages/features/booking-audit/lib/repository/IBookingAuditRepository.ts\n+++ b/packages/features/booking-audit/lib/repository/IBookingAuditRepository.ts\n@@ -55,4 +55,11 @@\n-    findAllForBooking(bookingUid: string): Promise<BookingAuditWithActor[]>;\n+    findAllForBooking(bookingUid: string): Promise<BookingAuditWithActor[]>;\n+\n+    findRescheduledLogsOfBooking(bookingUid: string): Promise<BookingAuditWithActor[]>;\n"
  },
  {
    "file": "packages/features/bookings/repositories/BookingRepository.ts",
    "hunk": "diff --git a/packages/features/bookings/repositories/BookingRepository.ts b/packages/features/bookings/repositories/BookingRepository.ts\n--- a/packages/features/bookings/repositories/BookingRepository.ts\n+++ b/packages/features/bookings/repositories/BookingRepository.ts\n@@ -321,7 +321,21 @@\n-export class BookingRepository {\n-  constructor(private prismaClient: PrismaClient) {}\n+export class BookingRepository {\n+  constructor(private prismaClient: PrismaClient) {}\n+\n+  async getFromRescheduleUid(bookingUid: string): Promise<string | null> {\n+    const booking = await this.prismaClient.booking.findUnique({\n+      where: { uid: bookingUid },\n+      select: { fromReschedule: true },\n+    });\n+    return booking?.fromReschedule ?? null;\n+  }\n"
  },
  {
    "file": "packages/features/di/containers/BookingAuditViewerService.container.ts",
    "hunk": "diff --git a/packages/features/di/containers/BookingAuditViewerService.container.ts b/packages/features/di/containers/BookingAuditViewerService.container.ts\nnew file mode 100644\n--- /dev/null\n+++ b/packages/features/di/containers/BookingAuditViewerService.container.ts\n@@ -0,0 +1,18 @@\n+import type { BookingAuditViewerService } from \"@calcom/features/booking-audit/lib/service/BookingAuditViewerService\";\n+import { BOOKING_AUDIT_DI_TOKENS } from \"@calcom/features/booking-audit/di/tokens\";\n+import { DI_TOKENS } from \"@calcom/features/di/tokens\";\n+import { prismaModule } from \"@calcom/features/di/modules/Prisma\";\n+import { moduleLoader as bookingAuditRepositoryModuleLoader } from \"@calcom/features/booking-audit/di/BookingAuditRepository.module\";\n+import { moduleLoader as bookingAuditViewerServiceModuleLoader } from \"@calcom/features/booking-audit/di/BookingAuditViewerService.module\";\n+\n+import { createContainer } from \"../di\";\n+\n+const container = createContainer();\n+container.load(DI_TOKENS.PRISMA_MODULE, prismaModule);\n+bookingAuditRepositoryModuleLoader.loadModule(container);\n+bookingAuditViewerServiceModuleLoader.loadModule(container);\n+\n+export function getBookingAuditViewerService() {\n+    return container.get<BookingAuditViewerService>(BOOKING_AUDIT_DI_TOKENS.BOOKING_AUDIT_VIEWER_SERVICE);\n+}\n+\n"
  }
]"#.trim().to_string(),
            ),
            diagram: Some(
                r#"direction: right

WebAPI: {
  label: "tRPC endpoint\nviewer.bookings.getAuditLogs"
}

Viewer: {
  label: BookingAuditViewerService
}

AuditRepo: {
  shape: cylinder
  label: "BookingAudit\nRepository"
}

BookingRepo: {
  shape: cylinder
  label: "Booking\nRepository"
}

Registry: {
  shape: hexagon
  label: "Action Service\nRegistry"
}

RescheduledSvc: {
  label: "Rescheduled\nAuditActionService"
}

WebAPI -> Viewer: "bookingUid\nuserTimeZone"
Viewer -> AuditRepo: findAllForBooking
Viewer -> BookingRepo: getFromRescheduleUid
BookingRepo -> Viewer: "fromRescheduleUid\nor null"
Viewer -> Registry: getActionService
Registry -> Viewer: action service
Viewer -> AuditRepo: "findRescheduled\nLogsOfBooking"
Viewer -> RescheduledSvc: "build rescheduled-from\ntitle"
Viewer -> WebAPI: "enriched logs with\nactionDisplayTitle\ndata, displayFields""#.trim().to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Viewer and reschedule context".to_string()),
        },

        // Actor helpers and DI wiring
        ReviewTask {
            id: "booking-audit-di-1".to_string(),
            pr_id: pr.id.clone(),
            title: "Review DI wiring, actor helpers, and repository extensions".to_string(),
            description: "Review the DI module changes and small helpers added around actors and booking repository. Confirm that BookingAuditTaskConsumer and BookingAuditViewerService receive the correct dependencies (userRepository, bookingRepository, logger, tasker), and that makeAttendeeActor and getFromRescheduleUid behave as expected. This task is mainly about wiring and future maintainability rather than deep business logic.".to_string(),
            files: r#"[
  "packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts",
  "packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts",
  "packages/features/booking-audit/di/BookingAuditViewerService.module.ts",
  "packages/features/bookings/lib/types/actor.ts",
  "packages/features/bookings/repositories/BookingRepository.ts"
]"#.to_string(),
            stats: r#"{"additions": 95, "deletions": 18, "risk": "LOW", "tags": ["di", "wiring", "booking-audit"]}"#.to_string(),
            insight: Some("These are the edges of the system. If a DI token or module wiring is wrong, audit logs will silently stop recording or viewing without obvious type errors. Treat this as a sanity check that the booking audit stack is reachable in real environments, not just in tests.".to_string()),
            patches: Some(
                r#"[
  {
    "file": "packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts",
    "hunk": "diff --git a/packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts b/packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts\n--- a/packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts\n+++ b/packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts\n@@ -3,6 +3,7 @@\n import { moduleLoader as bookingAuditRepositoryModuleLoader } from \"@calcom/features/booking-audit/di/BookingAuditRepository.module\";\n import { moduleLoader as auditActorRepositoryModuleLoader } from \"@calcom/features/booking-audit/di/AuditActorRepository.module\";\n import { moduleLoader as featuresRepositoryModuleLoader } from \"@calcom/features/di/modules/Features\";\n+import { moduleLoader as userRepositoryModuleLoader } from \"@calcom/features/di/modules/User\";\n@@\n-    featuresRepository: featuresRepositoryModuleLoader,\n+    featuresRepository: featuresRepositoryModuleLoader,\n+    userRepository: userRepositoryModuleLoader,\n   },\n });\n"
  },
  {
    "file": "packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts",
    "hunk": "diff --git a/packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts b/packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts\n--- a/packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts\n+++ b/packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts\n@@ -1,6 +1,7 @@\n import { BookingAuditTaskerProducerService } from \"@calcom/features/booking-audit/lib/service/BookingAuditTaskerProducerService\";\n import { BOOKING_AUDIT_DI_TOKENS } from \"@calcom/features/booking-audit/di/tokens\";\n import { moduleLoader as taskerModuleLoader } from \"@calcom/features/di/shared/services/tasker.service\";\n+import { moduleLoader as loggerModuleLoader } from \"@calcom/features/di/shared/services/logger.service\";\n@@\n   depsMap: {\n     tasker: taskerModuleLoader,\n+    log: loggerModuleLoader,\n   },\n });\n"
  },
  {
    "file": "packages/features/bookings/lib/types/actor.ts",
    "hunk": "diff --git a/packages/features/bookings/lib/types/actor.ts b/packages/features/bookings/lib/types/actor.ts\n--- a/packages/features/bookings/lib/types/actor.ts\n+++ b/packages/features/bookings/lib/types/actor.ts\n@@ -32,6 +32,7 @@\n export const ActorSchema = z.discriminatedUnion(\"identifiedBy\", [\n@@\n type UserActor = z.infer<typeof UserActorSchema>;\n type GuestActor = z.infer<typeof GuestActorSchema>;\n+type AttendeeActor = z.infer<typeof AttendeeActorSchema>;\n@@\n export function makeActorById(id: string): ActorById {\n@@\n }\n+\n+export function makeAttendeeActor(attendeeId: number): AttendeeActor {\n+  return {\n+    identifiedBy: \"attendee\",\n+    attendeeId,\n+  };\n+}\n"
  }
]"#.trim().to_string(),
            ),
            diagram: Some(
                r#"direction: down

Container: {
  label: "DI Container"
}

TaskerMod: {
  label: "Tasker Module"
}

LoggerMod: {
  label: "Logger Module"
}

ConsumerMod: {
  label: "BookingAuditTaskConsumer\nModule"
}

ProducerMod: {
  label: "BookingAuditTaskerProducerService\nModule"
}

ViewerMod: {
  label: "BookingAuditViewerService\nModule"
}

Container -> TaskerMod: load tasker
Container -> LoggerMod: load logger
Container -> ConsumerMod: "bind consumer deps:\nrepositories, features,\nuser repo"
Container -> ProducerMod: "bind producer deps:\ntasker, logger"
Container -> ViewerMod: "bind viewer deps:\naudit repo, user repo,\nbooking repo""#.trim().to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("DI and repositories".to_string()),
        },
    ];

    // Insert all tasks
    for task in tasks {
        conn.execute(
            r#"INSERT OR REPLACE INTO tasks (id, pull_request_id, title, description, files, stats, insight, patches, diagram, ai_generated, status, sub_flow) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            (
                &task.id,
                &task.pr_id,
                &task.title,
                &task.description,
                &task.files,
                &task.stats,
                &task.insight,
                &task.patches,
                &task.diagram,
                if task.ai_generated { 1 } else { 0 },
                &task.status,
                &task.sub_flow,
            ),
        )?;
        println!(
            "Inserted task: {} (Sub-flow: {})",
            task.title,
            task.sub_flow.as_deref().unwrap_or("None")
        );
    }

    println!("\nSample data successfully added to database!");
    println!("Database location: {}", db_path.display());
    println!(
        "Run the application with `cargo run` to see the intent-centric layout against this booking audit PR."
    );

    Ok(())
}
