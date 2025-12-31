use std::fs;

// Simple structs matching the database schema
#[derive(Debug)]
struct Review {
    id: String,
    title: String,
    summary: Option<String>,
    source_json: String,
    active_run_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug)]
struct ReviewRun {
    id: String,
    review_id: String,
    agent_id: String,
    input_ref: String,
    diff_text: String,
    diff_hash: String,
    created_at: String,
}

#[derive(Debug)]
struct ReviewTask {
    id: String,
    run_id: String,
    title: String,
    description: String,
    files: String, // JSON string
    stats: String, // JSON string
    insight: Option<String>,
    diff_refs: Option<String>,
    diagram: Option<String>,
    ai_generated: bool,
    status: String,
    sub_flow: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
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

    let db = lareview::infra::db::Database::open_at(db_path.clone())?;
    let conn = db.connection();
    let conn = conn.lock().unwrap();

    let review_id = "review-calcom-9821".to_string();
    let run_id = "run-1".to_string();

    // Sample Review based on cal.com booking audit and booking logs changes
    let review = Review {
        id: review_id.clone(),
        title: "Improve booking audit timeline and action registry".to_string(),
        summary: Some(
            "This PR improves the booking audit subsystem and the booking logs UI, introducing a centralized action registry, typed action data, and a richer timeline view.".to_string(),
        ),
        source_json: r#"{"type":"github_pr","owner":"calcom","repo":"cal.com","number":9821,"url":"https://github.com/calcom/cal.com/pull/9821"}"#.to_string(),
        active_run_id: Some(run_id.clone()),
        created_at: "2024-11-15T14:32:00Z".to_string(),
        updated_at: "2024-11-15T14:32:00Z".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO reviews (id, title, summary, source_json, active_run_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&review.id, &review.title, &review.summary, &review.source_json, &review.active_run_id, &review.created_at, &review.updated_at),
    )?;
    println!("Inserted Review: {}", review.title);

    // Realistic diff text for booking audit PR
    let diff_text = r#"diff --git a/apps/web/modules/booking/logs/views/booking-logs-view.tsx b/apps/web/modules/booking/logs/views/booking-logs-view.tsx
index abc1234..def5678 100644
--- a/apps/web/modules/booking/logs/views/booking-logs-view.tsx
+++ b/apps/web/modules/booking/logs/views/booking-logs-view.tsx
@@ -1,15 +1,32 @@
 import { useQuery } from "@tanstack/react-query";
+import { Avatar } from "@calcom/ui";
+import { ServerTrans } from "@calcom/lib/i18n";

 export function BookingLogsView({ bookingUid }: { bookingUid: string }) {
   const { data: logs } = useQuery({
     queryKey: ["booking-logs", bookingUid],
-    queryFn: () => trpc.viewer.bookings.getLogs.query({ bookingUid }),
+    queryFn: () => trpc.viewer.bookings.getAuditLogs.query({ bookingUid }),
   });

   return (
     <div className="space-y-4">
       {logs?.map((log) => (
-        <div key={log.id}>{log.message}</div>
+        <div key={log.id} className="flex items-start gap-3">
+          <Avatar
+            size="sm"
+            imageSrc={log.actor.avatarUrl}
+            alt={log.actor.name}
+          />
+          <div className="flex-1">
+            <ServerTrans
+              i18nKey={log.actionDisplayTitle.key}
+              values={log.actionDisplayTitle.params}
+              components={{ link: <a className="underline" /> }}
+            />
+            <JsonViewer data={log.displayFields} />
+          </div>
+        </div>
       ))}
     </div>
   );
diff --git a/apps/web/public/static/locales/en/common.json b/apps/web/public/static/locales/en/common.json
index 1234abc..5678def 100644
--- a/apps/web/public/static/locales/en/common.json
+++ b/apps/web/public/static/locales/en/common.json
@@ -850,6 +850,19 @@
   "booking_logs": "Booking Logs",
   "booking_audit": "Booking Audit",
+  "booking_audit_action": {
+    "created": "{{actor}} created this booking",
+    "rescheduled": "{{actor}} rescheduled from {{link}}",
+    "cancelled": "{{actor}} cancelled this booking",
+    "accepted": "{{actor}} accepted this booking",
+    "rejected": "{{actor}} rejected this booking",
+    "reassigned": "{{actor}} reassigned to {{assignee}}",
+    "reschedule_requested": "{{actor}} requested to reschedule",
+    "attendee_added": "{{actor}} added {{attendee}}",
+    "attendee_removed": "{{actor}} removed {{attendee}}",
+    "location_changed": "{{actor}} changed location",
+    "host_no_show": "{{actor}} marked host as no-show",
+    "attendee_no_show": "{{actor}} marked {{attendee}} as no-show"
+  },
   "view_booking_logs": "View booking logs"
 }
diff --git a/packages/features/booking-audit/lib/actions/IAuditActionService.ts b/packages/features/booking-audit/lib/actions/IAuditActionService.ts
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/packages/features/booking-audit/lib/actions/IAuditActionService.ts
@@ -0,0 +1,21 @@
+import { z } from "zod";
+import type { BookingAuditAction } from "@prisma/client";
+
+export interface IAuditActionService<TData extends z.ZodTypeAny> {
+  readonly action: BookingAuditAction;
+  readonly dataSchema: TData;
+
+  migrateToLatest(data: unknown, version: number): z.infer<TData>;
+
+  getDisplayTitle(data: z.infer<TData>, context: {
+    actor: { name: string };
+    userTimeZone: string;
+  }): {
+    key: string;
+    params: Record<string, any>;
+  };
+
+  getDisplayJson(data: z.infer<TData>): Record<string, any>;
+
+  getDisplayFields(data: z.infer<TData>): Array<{ label: string; value: string }>;
+}
diff --git a/packages/features/booking-audit/lib/actions/CreatedAuditActionService.ts b/packages/features/booking-audit/lib/actions/CreatedAuditActionService.ts
new file mode 100644
index 0000000..abcdef1
--- /dev/null
+++ b/packages/features/booking-audit/lib/actions/CreatedAuditActionService.ts
@@ -0,0 +1,35 @@
+import { z } from "zod";
+import { BookingAuditAction } from "@prisma/client";
+import type { IAuditActionService } from "./IAuditActionService";
+
+const CreatedDataSchema = z.object({
+  startTime: z.string(),
+  endTime: z.string(),
+  eventTypeId: z.number(),
+});
+
+export class CreatedAuditActionService implements IAuditActionService<typeof CreatedDataSchema> {
+  readonly action = BookingAuditAction.CREATED;
+  readonly dataSchema = CreatedDataSchema;
+
+  migrateToLatest(data: unknown, version: number) {
+    return this.dataSchema.parse(data);
+  }
+
+  getDisplayTitle(data: z.infer<typeof CreatedDataSchema>, context: { actor: { name: string } }) {
+    return {
+      key: "booking_audit_action.created",
+      params: { actor: context.actor.name },
+    };
+  }
+
+  getDisplayJson(data: z.infer<typeof CreatedDataSchema>) {
+    return data;
+  }
+
+  getDisplayFields(data: z.infer<typeof CreatedDataSchema>) {
+    return [
+      { label: "Event Type ID", value: data.eventTypeId.toString() },
+    ];
+  }
+}
diff --git a/packages/features/booking-audit/lib/actions/RescheduledAuditActionService.ts b/packages/features/booking-audit/lib/actions/RescheduledAuditActionService.ts
new file mode 100644
index 0000000..9876543
--- /dev/null
+++ b/packages/features/booking-audit/lib/actions/RescheduledAuditActionService.ts
@@ -0,0 +1,40 @@
+import { z } from "zod";
+import { BookingAuditAction } from "@prisma/client";
+import type { IAuditActionService } from "./IAuditActionService";
+import { BookingStatusChangeSchema } from "../common/changeSchemas";
+
+const RescheduledDataSchema = BookingStatusChangeSchema.extend({
+  fromBookingUid: z.string(),
+  oldStartTime: z.string(),
+  newStartTime: z.string(),
+});
+
+export class RescheduledAuditActionService implements IAuditActionService<typeof RescheduledDataSchema> {
+  readonly action = BookingAuditAction.RESCHEDULED;
+  readonly dataSchema = RescheduledDataSchema;
+
+  migrateToLatest(data: unknown, version: number) {
+    return this.dataSchema.parse(data);
+  }
+
+  getDisplayTitle(data: z.infer<typeof RescheduledDataSchema>, context: { actor: { name: string } }) {
+    return {
+      key: "booking_audit_action.rescheduled",
+      params: {
+        actor: context.actor.name,
+        link: `/booking/${data.fromBookingUid}`,
+      },
+    };
+  }
+
+  getDisplayJson(data: z.infer<typeof RescheduledDataSchema>) {
+    return data;
+  }
+
+  getDisplayFields(data: z.infer<typeof RescheduledDataSchema>) {
+    return [
+      { label: "From", value: data.oldStartTime },
+      { label: "To", value: data.newStartTime },
+    ];
+  }
+}
diff --git a/packages/features/booking-audit/lib/common/changeSchemas.ts b/packages/features/booking-audit/lib/common/changeSchemas.ts
new file mode 100644
index 0000000..2468ace
--- /dev/null
+++ b/packages/features/booking-audit/lib/common/changeSchemas.ts
@@ -0,0 +1,11 @@
+import { z } from "zod";
+import { BookingStatus } from "@prisma/client";
+
+export const BookingStatusChangeSchema = z.object({
+  oldStatus: z.nativeEnum(BookingStatus).optional(),
+  newStatus: z.nativeEnum(BookingStatus),
+});
+
+export const AttendeeChangeSchema = z.object({
+  attendeeEmail: z.string().email(),
+});
diff --git a/packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts b/packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts
new file mode 100644
index 0000000..fedcba9
--- /dev/null
+++ b/packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts
@@ -0,0 +1,48 @@
+import { BookingAuditAction } from "@prisma/client";
+import type { IAuditActionService } from "../actions/IAuditActionService";
+import { CreatedAuditActionService } from "../actions/CreatedAuditActionService";
+import { RescheduledAuditActionService } from "../actions/RescheduledAuditActionService";
+import { CancelledAuditActionService } from "../actions/CancelledAuditActionService";
+import { AcceptedAuditActionService } from "../actions/AcceptedAuditActionService";
+import { RejectedAuditActionService } from "../actions/RejectedAuditActionService";
+import { ReassignmentAuditActionService } from "../actions/ReassignmentAuditActionService";
+import { RescheduleRequestedAuditActionService } from "../actions/RescheduleRequestedAuditActionService";
+import { AttendeeAddedAuditActionService } from "../actions/AttendeeAddedAuditActionService";
+import { AttendeeRemovedAuditActionService } from "../actions/AttendeeRemovedAuditActionService";
+import { LocationChangedAuditActionService } from "../actions/LocationChangedAuditActionService";
+import { HostNoShowUpdatedAuditActionService } from "../actions/HostNoShowUpdatedAuditActionService";
+import { AttendeeNoShowUpdatedAuditActionService } from "../actions/AttendeeNoShowUpdatedAuditActionService";
+
+export class BookingAuditActionServiceRegistry {
+  private readonly actionServices: Map<BookingAuditAction, IAuditActionService<any>>;
+
+  constructor() {
+    this.actionServices = new Map([
+      [BookingAuditAction.CREATED, new CreatedAuditActionService()],
+      [BookingAuditAction.RESCHEDULED, new RescheduledAuditActionService()],
+      [BookingAuditAction.CANCELLED, new CancelledAuditActionService()],
+      [BookingAuditAction.ACCEPTED, new AcceptedAuditActionService()],
+      [BookingAuditAction.REJECTED, new RejectedAuditActionService()],
+      [BookingAuditAction.REASSIGNMENT, new ReassignmentAuditActionService()],
+      [BookingAuditAction.RESCHEDULE_REQUESTED, new RescheduleRequestedAuditActionService()],
+      [BookingAuditAction.ATTENDEE_ADDED, new AttendeeAddedAuditActionService()],
+      [BookingAuditAction.ATTENDEE_REMOVED, new AttendeeRemovedAuditActionService()],
+      [BookingAuditAction.LOCATION_CHANGED, new LocationChangedAuditActionService()],
+      [BookingAuditAction.HOST_NO_SHOW, new HostNoShowUpdatedAuditActionService()],
+      [BookingAuditAction.ATTENDEE_NO_SHOW, new AttendeeNoShowUpdatedAuditActionService()],
+    ]);
+  }
+
+  getActionService(action: BookingAuditAction): IAuditActionService<any> {
+    const service = this.actionServices.get(action);
+    if (!service) {
+      throw new Error(`No action service registered for action: ${action}`);
+    }
+    return service;
+  }
+
+  getAllActions(): BookingAuditAction[] {
+    return Array.from(this.actionServices.keys());
+  }
+}
diff --git a/packages/features/booking-audit/lib/types/bookingAuditTask.ts b/packages/features/booking-audit/lib/types/bookingAuditTask.ts
new file mode 100644
index 0000000..7654321
--- /dev/null
+++ b/packages/features/booking-audit/lib/types/bookingAuditTask.ts
@@ -0,0 +1,18 @@
+import { z } from "zod";
+import { BookingAuditAction } from "@prisma/client";
+
+export const BookingAuditTaskBasePayload = z.object({
+  action: z.nativeEnum(BookingAuditAction),
+  bookingId: z.number(),
+  organizationId: z.number().nullable(),
+  actorId: z.number(),
+  data: z.unknown(),
+  version: z.number().default(1),
+});
+
+export type BookingAuditTaskPayload = z.infer<typeof BookingAuditTaskBasePayload>;
+
+export const BOOKING_AUDIT_TASK_NAME = "bookingAudit" as const;
+
+export type BookingAuditTaskName = typeof BOOKING_AUDIT_TASK_NAME;
diff --git a/packages/features/booking-audit/lib/service/BookingAuditTaskConsumer.ts b/packages/features/booking-audit/lib/service/BookingAuditTaskConsumer.ts
new file mode 100644
index 0000000..8901234
--- /dev/null
+++ b/packages/features/booking-audit/lib/service/BookingAuditTaskConsumer.ts
@@ -0,0 +1,45 @@
+import type { Logger } from "@calcom/lib/logger";
+import type { IBookingAuditRepository } from "../repository/IBookingAuditRepository";
+import { BookingAuditActionServiceRegistry } from "./BookingAuditActionServiceRegistry";
+import { BookingAuditTaskBasePayload } from "../types/bookingAuditTask";
+
+export class BookingAuditTaskConsumer {
+  constructor(
+    private readonly repository: IBookingAuditRepository,
+    private readonly registry: BookingAuditActionServiceRegistry,
+    private readonly logger: Logger
+  ) {}
+
+  async consume(payload: unknown): Promise<void> {
+    try {
+      const parsed = BookingAuditTaskBasePayload.parse(payload);
+
+      const actionService = this.registry.getActionService(parsed.action);
+
+      const migratedData = actionService.migrateToLatest(parsed.data, parsed.version);
+
+      await this.repository.create({
+        action: parsed.action,
+        bookingId: parsed.bookingId,
+        organizationId: parsed.organizationId,
+        actorId: parsed.actorId,
+        data: migratedData,
+        version: parsed.version,
+      });
+
+      this.logger.info("Booking audit log created", {
+        action: parsed.action,
+        bookingId: parsed.bookingId,
+      });
+    } catch (error) {
+      this.logger.error("Failed to consume booking audit task", {
+        error,
+        payload,
+      });
+
+      if (error instanceof Error && error.message.includes("No action service")) {
+        throw error;
+      }
+    }
+  }
+}
diff --git a/packages/features/booking-audit/lib/service/BookingAuditTaskerProducerService.ts b/packages/features/booking-audit/lib/service/BookingAuditTaskerProducerService.ts
new file mode 100644
index 0000000..3456789
--- /dev/null
+++ b/packages/features/booking-audit/lib/service/BookingAuditTaskerProducerService.ts
@@ -0,0 +1,48 @@
+import type { Logger } from "@calcom/lib/logger";
+import type { Tasker } from "@calcom/features/tasker";
+import { BookingAuditAction } from "@prisma/client";
+import { BOOKING_AUDIT_TASK_NAME, BookingAuditTaskPayload } from "../types/bookingAuditTask";
+import type { IBookingAuditProducerService } from "./BookingAuditProducerService.interface";
+
+export class BookingAuditTaskerProducerService implements IBookingAuditProducerService {
+  constructor(
+    private readonly tasker: Tasker,
+    private readonly logger: Logger
+  ) {}
+
+  async queueCreatedAudit(params: {
+    bookingId: number;
+    organizationId: number | null;
+    actorId: number;
+    data: unknown;
+  }): Promise<void> {
+    await this.queueAudit(BookingAuditAction.CREATED, params);
+  }
+
+  async queueRescheduledAudit(params: {
+    bookingId: number;
+    organizationId: number | null;
+    actorId: number;
+    data: unknown;
+  }): Promise<void> {
+    await this.queueAudit(BookingAuditAction.RESCHEDULED, params);
+  }
+
+  private async queueAudit(
+    action: BookingAuditAction,
+    params: { bookingId: number; organizationId: number | null; actorId: number; data: unknown }
+  ): Promise<void> {
+    const payload: BookingAuditTaskPayload = {
+      action,
+      bookingId: params.bookingId,
+      organizationId: params.organizationId,
+      actorId: params.actorId,
+      data: params.data,
+      version: 1,
+    };
+
+    await this.tasker.create(BOOKING_AUDIT_TASK_NAME, payload);
+
+    this.logger.info("Queued booking audit task", { action, bookingId: params.bookingId });
+  }
+}
diff --git a/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts b/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts
new file mode 100644
index 0000000..5432109
--- /dev/null
+++ b/packages/features/booking-audit/lib/service/BookingAuditViewerService.ts
@@ -0,0 +1,67 @@
+import type { IBookingAuditRepository } from "../repository/IBookingAuditRepository";
+import type { IBookingRepository } from "@calcom/features/bookings/repositories/BookingRepository";
+import { BookingAuditActionServiceRegistry } from "./BookingAuditActionServiceRegistry";
+import { BookingAuditAction } from "@prisma/client";
+
+export class BookingAuditViewerService {
+  constructor(
+    private readonly auditRepository: IBookingAuditRepository,
+    private readonly bookingRepository: IBookingRepository,
+    private readonly registry: BookingAuditActionServiceRegistry
+  ) {}
+
+  async getAuditLogsForBooking(bookingUid: string, userTimeZone: string) {
+    const logs = await this.auditRepository.findAllForBooking(bookingUid);
+
+    const booking = await this.bookingRepository.findByUid(bookingUid);
+    const fromRescheduleUid = await this.bookingRepository.getFromRescheduleUid(booking?.id);
+
+    const enrichedLogs = [];
+
+    if (fromRescheduleUid) {
+      const previousLogs = await this.auditRepository.findRescheduledLogsOfBooking(fromRescheduleUid);
+      const rescheduledService = this.registry.getActionService(BookingAuditAction.RESCHEDULED);
+
+      if (previousLogs.length > 0) {
+        const syntheticLog = {
+          id: `synthetic-rescheduled-from-${fromRescheduleUid}`,
+          action: BookingAuditAction.RESCHEDULED,
+          actor: previousLogs[0].actor,
+          actionDisplayTitle: rescheduledService.getDisplayTitle(
+            { fromBookingUid: fromRescheduleUid },
+            { actor: previousLogs[0].actor, userTimeZone }
+          ),
+          displayFields: rescheduledService.getDisplayFields({ fromBookingUid: fromRescheduleUid }),
+          createdAt: previousLogs[0].createdAt,
+        };
+        enrichedLogs.push(syntheticLog);
+      }
+    }
+
+    for (const log of logs) {
+      const actionService = this.registry.getActionService(log.action);
+
+      enrichedLogs.push({
+        id: log.id,
+        action: log.action,
+        actor: log.actor,
+        actionDisplayTitle: actionService.getDisplayTitle(log.data, {
+          actor: log.actor,
+          userTimeZone,
+        }),
+        displayFields: actionService.getDisplayFields(log.data),
+        data: actionService.getDisplayJson(log.data),
+        createdAt: log.createdAt,
+      });
+    }
+
+    return enrichedLogs.sort((a, b) =>
+      new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
+    );
+  }
+}
diff --git a/packages/features/bookings/repositories/BookingRepository.ts b/packages/features/bookings/repositories/BookingRepository.ts
index 1111111..2222222 100644
--- a/packages/features/bookings/repositories/BookingRepository.ts
+++ b/packages/features/bookings/repositories/BookingRepository.ts
@@ -50,6 +50,16 @@ export class BookingRepository implements IBookingRepository {
     });
   }

+  async getFromRescheduleUid(bookingId: number | undefined): Promise<string | null> {
+    if (!bookingId) return null;
+
+    const booking = await this.prisma.booking.findUnique({
+      where: { id: bookingId },
+      select: { fromReschedule: true },
+    });
+
+    return booking?.fromReschedule ?? null;
+  }
+
   async update(id: number, data: Partial<Booking>): Promise<Booking> {
     return this.prisma.booking.update({
       where: { id },
diff --git a/packages/features/tasker/tasker.ts b/packages/features/tasker/tasker.ts
index aaa1111..bbb2222 100644
--- a/packages/features/tasker/tasker.ts
+++ b/packages/features/tasker/tasker.ts
@@ -1,5 +1,6 @@
 import type { TaskPayload } from "./types";
 import { Logger } from "@calcom/lib/logger";
+import { BOOKING_AUDIT_TASK_NAME } from "@calcom/features/booking-audit/lib/types/bookingAuditTask";

 export interface Tasker {
   create(taskName: string, payload: TaskPayload): Promise<void>;
@@ -15,6 +16,9 @@ export class TaskerImpl implements Tasker {

   async create(taskName: string, payload: TaskPayload): Promise<void> {
     this.logger.info("Creating task", { taskName, payload });
+
+    if (taskName === BOOKING_AUDIT_TASK_NAME) {
+      await this.processBookingAuditTask(payload);
+    }
   }

   async process(taskName: string): Promise<void> {
"#;

    let review_run = ReviewRun {
        id: run_id.clone(),
        review_id: review.id.clone(),
        agent_id: "gemini".to_string(),
        input_ref: "https://github.com/calcom/cal.com/pull/9821".to_string(),
        diff_text: diff_text.to_string(),
        diff_hash: "sha256_9821_booking_audit".to_string(),
        created_at: "2024-11-15T14:35:00Z".to_string(),
    };

    conn.execute(
        "INSERT OR REPLACE INTO review_runs (id, review_id, agent_id, input_ref, diff_text, diff_hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&review_run.id, &review_run.review_id, &review_run.agent_id, &review_run.input_ref, &review_run.diff_text, &review_run.diff_hash, &review_run.created_at),
    )?;
    println!("Inserted Review Run for review: {}", review.title);

    // Sample tasks modeled after real review items from the cal.com diff
    let tasks = vec![
        // Booking logs UI and i18n
        ReviewTask {
            id: "booking-logs-ui-1".to_string(),
            run_id: run_id.clone(),
            title: "Review booking logs timeline UI and i18n wiring".to_string(),
            description: "Review the changes in the booking logs timeline UI. Confirm that the new ActionTitle and JsonViewer components integrate correctly with the tRPC response shape, that avatars and actor roles are rendered safely, and that we do not regress accessibility or i18n behavior. Pay attention to how booking_audit_action.* keys are used and how link components are passed through ServerTrans.".to_string(),
            files: r#"[
  "apps/web/modules/booking/logs/views/booking-logs-view.tsx",
  "apps/web/public/static/locales/en/common.json"
]"#
            .to_string(),
            stats: r#"{"additions": 210, "deletions": 75, "risk": "MEDIUM", "tags": ["booking-logs", "react", "i18n", "ui"]}"#.to_string(),
            insight: Some("This UI is a primary debugging surface for support and customers. A small regression in how we format dates, JSON, or links will be very visible. Treat the JsonViewer as a mini log viewer and think about default collapsed behavior and performance on large payloads.".to_string()),
            diff_refs: Some(serde_json::to_string(&[
                serde_json::json!({
                    "file": "apps/web/modules/booking/logs/views/booking-logs-view.tsx",
                    "hunks": [
                        {
                            "old_start": 1,
                            "old_lines": 15,
                            "new_start": 1,
                            "new_lines": 32
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "apps/web/public/static/locales/en/common.json",
                    "hunks": [
                        {
                            "old_start": 850,
                            "old_lines": 2,
                            "new_start": 850,
                            "new_lines": 15
                        }
                    ]
                })
            ])?),
            diagram: Some(
                r#"{
  "type": "flow",
  "data": {
    "direction": "LR",
    "nodes": [
      { "id": "client", "label": "User", "kind": "user" },
      { "id": "web_app", "label": "Next.js BookingLogsView", "kind": "service" },
      { "id": "api", "label": "tRPC getAuditLogs", "kind": "service" },
      { "id": "viewer_service", "label": "BookingAuditViewerService", "kind": "service" },
      { "id": "repos", "label": "Repositories", "kind": "database" }
    ],
    "edges": [
      { "from": "client", "to": "web_app", "label": "Open booking logs" },
      { "from": "web_app", "to": "api", "label": "getAuditLogs" },
      { "from": "api", "to": "viewer_service", "label": "Fetch audit data" },
      { "from": "viewer_service", "to": "repos", "label": "Load logs & metadata" },
      { "from": "repos", "to": "viewer_service", "label": "Raw audit records" },
      { "from": "viewer_service", "to": "api", "label": "Enriched logs" },
      { "from": "api", "to": "web_app", "label": "JSON response" },
      { "from": "web_app", "to": "client", "label": "Render timeline" }
    ]
  }
}"#
                .to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Booking logs UI".to_string()),
        },

        // Action services, schemas, and registry
        ReviewTask {
            id: "booking-audit-actions-1".to_string(),
            run_id: run_id.clone(),
            title: "Review booking audit action services and registry".to_string(),
            description: "Review the BookingAuditActionServiceRegistry and the action services under packages/features/booking-audit/lib/actions. Confirm that all BookingAuditAction variants are wired into the registry, that migrateToLatest, getDisplayTitle, and getDisplayJson, and getDisplayFields contracts are consistent with IAuditActionService, and that BookingStatusChangeSchema is used where appropriate. Pay attention to how translation keys and params are produced for frontend use.".to_string(),
            files: r#"[
  "packages/features/booking-audit/lib/actions/IAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/CreatedAuditActionService.ts",
  "packages/features/booking-audit/lib/actions/RescheduledAuditActionService.ts",
  "packages/features/booking-audit/lib/common/changeSchemas.ts",
  "packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts"
]"#
            .to_string(),
            stats: r#"{"additions": 260, "deletions": 40, "risk": "HIGH", "tags": ["booking-audit", "typescript", "domain-model", "i18n"]}"#.to_string(),
            insight: Some("These action services define the contract between producers, consumers, and the booking logs UI. If the registry mapping or zod schemas drift, we will either drop logs or misrender titles. Review the registry as a single source of truth and think about how we would add a new action in the future without touching too many files.".to_string()),
            diff_refs: Some(serde_json::to_string(&[
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/actions/IAuditActionService.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 21
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/actions/CreatedAuditActionService.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 35
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/actions/RescheduledAuditActionService.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 40
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/common/changeSchemas.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 11
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/service/BookingAuditActionServiceRegistry.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 48
                        }
                    ]
                })
            ])?),
            diagram: Some(
                r#"{
  "type": "flow",
  "data": {
    "direction": "LR",
    "nodes": [
      { "id": "producer", "label": "Producer Service", "kind": "service" },
      { "id": "tasker", "label": "Tasker Queue", "kind": "queue" },
      { "id": "consumer", "label": "Task Consumer", "kind": "service" },
      { "id": "registry", "label": "Action Registry", "kind": "lambda" },
      { "id": "action_services", "label": "Action Services", "kind": "service" },
      { "id": "repo", "label": "Audit Repository", "kind": "database" }
    ],
    "edges": [
      { "from": "producer", "to": "tasker", "label": "Queue audit" },
      { "from": "producer", "to": "tasker", "label": "Enqueue task" },
      { "from": "tasker", "to": "consumer", "label": "Deliver payload" },
      { "from": "consumer", "to": "registry", "label": "Get service" },
      { "from": "registry", "to": "action_services", "label": "Route to action" },
      { "from": "action_services", "to": "consumer", "label": "Process & migrate" },
      { "from": "consumer", "to": "repo", "label": "Store versioned data" }
    ]
  }
}"#
                .to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Action services and registry".to_string()),
        },

        // Task consumer, producer, and task payload
        ReviewTask {
            id: "booking-audit-tasks-1".to_string(),
            run_id: run_id.clone(),
            title: "Review booking audit task consumer and producer pipeline".to_string(),
            description: "Review the changes in BookingAuditTaskConsumer, BookingAuditTaskerProducerService, bookingAuditTask types, and tasker.ts. Confirm that the lean BookingAuditTaskBasePayload is sufficient for routing, that action-specific zod validation occurs in the right place, and that legacy queueAudit usage remains safe. Pay attention to how errors are logged, how organizationId null cases are handled, and how IS_PRODUCTION influences queueing behavior.".to_string(),
            files: r#"[
  "packages/features/booking-audit/lib/types/bookingAuditTask.ts",
  "packages/features/booking-audit/lib/service/BookingAuditTaskConsumer.ts",
  "packages/features/booking-audit/lib/service/BookingAuditTaskerProducerService.ts",
  "packages/features/tasker/tasker.ts"
]"#
            .to_string(),
            stats: r#"{"additions": 310, "deletions": 120, "risk": "HIGH", "tags": ["tasker", "queue", "booking-audit", "typescript"]}"#.to_string(),
            insight: Some("This is a good place to think about failure modes. What happens if an action is added to the enum but not to the registry, or vice versa. What happens if a producer passes the wrong data shape. Consider logging, observability, and how we might backfill or replay audit tasks if something goes wrong.".to_string()),
            diff_refs: Some(serde_json::to_string(&[
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/types/bookingAuditTask.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 18
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/service/BookingAuditTaskConsumer.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 45
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/service/BookingAuditTaskerProducerService.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 48
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/tasker/tasker.ts",
                    "hunks": [
                        {
                            "old_start": 1,
                            "old_lines": 5,
                            "new_start": 1,
                            "new_lines": 6
                        },
                        {
                            "old_start": 15,
                            "old_lines": 2,
                            "new_start": 16,
                            "new_lines": 7
                        }
                    ]
                })
            ])?),
            diagram: Some(
                r#"{
  "type": "flow",
  "data": {
    "direction": "LR",
    "nodes": [
      { "id": "api", "label": "Booking Service", "kind": "service" },
      { "id": "producer", "label": "BookingAuditTasker ProducerService", "kind": "service" },
      { "id": "tasker", "label": "Tasker bookingAudit", "kind": "queue" },
      { "id": "consumer", "label": "BookingAuditTask Consumer", "kind": "service" },
      { "id": "registry", "label": "Action Service Registry", "kind": "lambda" },
      { "id": "repo", "label": "BookingAudit Repository", "kind": "database" }
    ],
    "edges": [
      { "from": "api", "to": "producer", "label": "queueCreatedAudit, queueRescheduledAudit" },
      { "from": "producer", "to": "tasker", "label": "create task with base payload" },
      { "from": "tasker", "to": "consumer", "label": "deliver payload" },
      { "from": "consumer", "to": "registry", "label": "getActionService" },
      { "from": "registry", "to": "consumer", "label": "return typed service" },
      { "from": "consumer", "to": "repo", "label": "insert audit row with versioned data" }
    ]
  }
}"#
                .to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Task processing pipeline".to_string()),
        },

        // Viewer service and reschedule context
        ReviewTask {
            id: "booking-audit-viewer-1".to_string(),
            run_id: run_id.clone(),
            title: "Review BookingAuditViewerService and reschedule context handling".to_string(),
            description: "Review BookingAuditViewerService and the new container module. Confirm that getAuditLogsForBooking enriches actors correctly, calls getDisplayTitle and getDisplayJson on the right action services, and handles missing or malformed data defensively. Pay special attention to the rescheduled from logic that pulls RESCHEDULED logs from the previous booking and injects a synthetic entry at the top of the timeline for the current booking.".to_string(),
            files: r#"[
  "packages/features/booking-audit/lib/service/BookingAuditViewerService.ts",
  "packages/features/bookings/repositories/BookingRepository.ts"
]"#
            .to_string(),
            stats: r#"{"additions": 230, "deletions": 60, "risk": "MEDIUM", "tags": ["booking-audit", "viewer", "typescript"]}"#.to_string(),
            insight: Some("The viewer service is the bridge between storage and UI. The new 'rescheduled from' synthetic log is a good place to look for off-by-one style bugs or confusing ownership of bookingUid. Think about how this behaves for long reschedule chains and how we might test that in isolation.".to_string()),
            diff_refs: Some(serde_json::to_string(&[
                serde_json::json!({
                    "file": "packages/features/booking-audit/lib/service/BookingAuditViewerService.ts",
                    "hunks": [
                        {
                            "old_start": 0,
                            "old_lines": 0,
                            "new_start": 1,
                            "new_lines": 67
                        }
                    ]
                }),
                serde_json::json!({
                    "file": "packages/features/bookings/repositories/BookingRepository.ts",
                    "hunks": [
                        {
                            "old_start": 50,
                            "old_lines": 3,
                            "new_start": 50,
                            "new_lines": 13
                        }
                    ]
                })
            ])?),
            diagram: Some(
                r#"{
  "type": "flow",
  "data": {
    "direction": "LR",
    "nodes": [
      { "id": "web_api", "label": "tRPC endpoint tviewer.bookings.getAuditLogs", "kind": "service" },
      { "id": "viewer", "label": "BookingAuditViewerService", "kind": "service" },
      { "id": "audit_repo", "label": "BookingAudit Repository", "kind": "database" },
      { "id": "booking_repo", "label": "Booking Repository", "kind": "database" },
      { "id": "registry", "label": "Action Service Registry", "kind": "lambda" },
      { "id": "rescheduled_svc", "label": "Rescheduled AuditActionService", "kind": "service" }
    ],
    "edges": [
      { "from": "web_api", "to": "viewer", "label": "bookingUid, userTimeZone" },
      { "from": "viewer", "to": "audit_repo", "label": "findAllForBooking" },
      { "from": "viewer", "to": "booking_repo", "label": "getFromRescheduleUid" },
      { "from": "booking_repo", "to": "viewer", "label": "fromRescheduleUid or null" },
      { "from": "viewer", "to": "registry", "label": "getActionService" },
      { "from": "registry", "to": "viewer", "label": "action service" },
      { "from": "viewer", "to": "audit_repo", "label": "findRescheduled logs of booking" },
      { "from": "viewer", "to": "rescheduled_svc", "label": "build rescheduled-from title" },
      { "from": "viewer", "to": "web_api", "label": "enriched logs with actionDisplayTitle data, displayFields" }
    ]
  }
}"#
                .to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("Viewer and reschedule context".to_string()),
        },

        // Actor helpers and DI wiring
        ReviewTask {
            id: "booking-audit-di-1".to_string(),
            run_id: run_id.clone(),
            title: "Review DI wiring, actor helpers, and repository extensions".to_string(),
            description: "Review the DI module changes and small helpers added around actors and booking repository. Confirm that BookingAuditTaskConsumer and BookingAuditViewerService receive the correct dependencies (userRepository, bookingRepository, logger, tasker), and that makeAttendeeActor and getFromRescheduleUid behave as expected. This task is mainly about wiring and future maintainability rather than deep business logic.".to_string(),
            files: r#"[
  "packages/features/booking-audit/di/BookingAuditTaskConsumer.module.ts",
  "packages/features/booking-audit/di/BookingAuditTaskerProducerService.module.ts",
  "packages/features/booking-audit/di/BookingAuditViewerService.module.ts",
  "packages/features/bookings/lib/types/actor.ts",
  "packages/features/bookings/repositories/BookingRepository.ts"
]"#
            .to_string(),
            stats: r#"{"additions": 95, "deletions": 18, "risk": "LOW", "tags": ["di", "wiring", "booking-audit"]}"#.to_string(),
            insight: Some("These are the edges of the system. If a DI token or module wiring is wrong, audit logs will silently stop recording or viewing without obvious type errors. Treat this as a sanity check that the booking audit stack is reachable in real environments, not just in tests.".to_string()),
            diff_refs: None, // DI wiring typically doesn't reference specific hunks
            diagram: Some(
                r#"{
  "type": "flow",
  "data": {
    "direction": "TB",
    "nodes": [
      { "id": "container", "label": "DI Container", "kind": "service" },
      { "id": "tasker_mod", "label": "Tasker Module", "kind": "service" },
      { "id": "logger_mod", "label": "Logger Module", "kind": "service" },
      { "id": "consumer_mod", "label": "BookingAuditTaskConsumer Module", "kind": "service" },
      { "id": "producer_mod", "label": "BookingAuditTaskerProducerService Module", "kind": "service" },
      { "id": "viewer_mod", "label": "BookingAuditViewerService Module", "kind": "service" }
    ],
    "edges": [
      { "from": "container", "to": "tasker_mod", "label": "load tasker" },
      { "from": "container", "to": "logger_mod", "label": "load logger" },
      { "from": "container", "to": "consumer_mod", "label": "bind consumer deps: repositories, features, user repo" },
      { "from": "container", "to": "producer_mod", "label": "bind producer deps: tasker, logger" },
      { "from": "container", "to": "viewer_mod", "label": "bind viewer deps: audit repo, user repo, booking repo" }
    ]
  }
}"#
                .to_string(),
            ),
            ai_generated: true,
            status: "PENDING".to_string(),
            sub_flow: Some("DI and repositories".to_string()),
        },
    ];

    // Insert all tasks
    for task in tasks {
        conn.execute(
            r#"INSERT OR REPLACE INTO tasks (id, run_id, title, description, files, stats, insight, diff_refs, diagram, ai_generated, status, sub_flow) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            (
                &task.id,
                &task.run_id,
                &task.title,
                &task.description,
                &task.files,
                &task.stats,
                &task.insight,
                &task.diff_refs,
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    #[test]
    fn test_seed_db_run() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        unsafe {
            std::env::set_var("LAREVIEW_DB_PATH", &path);
        }

        run().unwrap();

        let conn = Connection::open(&path).unwrap();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM reviews", [], |row| {
                row.get::<_, i32>(0)
            })
            .unwrap();
        assert!(count > 0);

        unsafe {
            std::env::remove_var("LAREVIEW_DB_PATH");
        }
    }
}
