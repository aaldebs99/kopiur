/**
 * The list tables' columns, one declaration per table: what each shows, how
 * wide, and which sort. Here rather than beside each table so the sorting
 * test can read them, and a component file exports only components.
 */
import type { ColumnSpec } from "./tableColumns";

export type RepositoryColumn =
  | "repository"
  | "health"
  | "phase"
  | "access"
  | "backend"
  | "snapshots"
  | "size"
  | "indexBlobs"
  | "lastObserved";

export const REPOSITORY_COLUMNS: readonly ColumnSpec<RepositoryColumn>[] = [
  {
    id: "repository",
    label: "Repository",
    width: 240,
    min: 240,
    locked: true,
    stripe: true,
    sort: "text",
  },
  { id: "health", label: "Health", width: 130, min: 130, sort: "state" },
  { id: "phase", label: "Phase", width: 120, min: 80, sort: "text" },
  { id: "access", label: "Access", width: 180, min: 150 },
  { id: "backend", label: "Backend", width: 110, min: 90, sort: "text" },
  { id: "snapshots", label: "Snapshots", width: 110, min: 110, numeric: true, sort: "number" },
  { id: "size", label: "Size", width: 100, min: 96, numeric: true, sort: "number" },
  {
    id: "indexBlobs",
    label: "Index blobs",
    width: 120,
    min: 120,
    numeric: true,
    sort: "number",
  },
  { id: "lastObserved", label: "Last observed", width: 150, min: 130, sort: "time" },
];

export type PolicyColumn =
  | "policy"
  | "writesInto"
  | "state"
  | "lastSnapshot"
  | "lastVerified"
  | "snapshots";

export const POLICY_COLUMNS: readonly ColumnSpec<PolicyColumn>[] = [
  { id: "policy", label: "Policy", width: 300, min: 220, locked: true, stripe: true, sort: "text" },
  { id: "writesInto", label: "Writes into", width: 260, min: 220 },
  { id: "state", label: "State", width: 140, min: 140, sort: "state" },
  { id: "lastSnapshot", label: "Last snapshot", width: 180, min: 180, sort: "time" },
  { id: "lastVerified", label: "Last verified", width: 180, min: 180, sort: "time" },
  { id: "snapshots", label: "Snapshots", width: 110, min: 110, numeric: true, sort: "number" },
];

export type ScheduleColumn =
  | "schedule"
  | "cron"
  | "fires"
  | "state"
  | "lastFire"
  | "nextFire"
  | "action";

export const SCHEDULE_COLUMNS: readonly ColumnSpec<ScheduleColumn>[] = [
  {
    id: "schedule",
    label: "Schedule",
    width: 240,
    min: 220,
    locked: true,
    stripe: true,
    sort: "text",
  },
  { id: "cron", label: "Cron", width: 200, min: 200, className: "mono schedule-table__cron" },
  { id: "fires", label: "Fires", width: 220, min: 160 },
  { id: "state", label: "State", width: 150, min: 140, sort: "state" },
  { id: "lastFire", label: "Last fire", width: 150, min: 150, numeric: true, sort: "time" },
  { id: "nextFire", label: "Next fire", width: 170, min: 170, numeric: true, sort: "time" },
];

/** With the route's suspend control: a column of buttons, kept last. */
export const SCHEDULE_COLUMNS_WITH_ACTION: readonly ColumnSpec<ScheduleColumn>[] = [
  ...SCHEDULE_COLUMNS,
  {
    id: "action",
    label: "Action",
    width: 150,
    min: 150,
    locked: true,
    resizable: false,
    className: "schedule-table__action",
  },
];

export type RestoreColumn = "restore" | "phase" | "route" | "repository" | "restored" | "started";

export const RESTORE_COLUMNS: readonly ColumnSpec<RestoreColumn>[] = [
  {
    id: "restore",
    label: "Restore",
    width: 300,
    min: 220,
    locked: true,
    stripe: true,
    sort: "text",
  },
  { id: "phase", label: "Phase", width: 140, min: 130, sort: "state" },
  { id: "route", label: "Reads → writes", width: 220, min: 140 },
  { id: "repository", label: "Repository", width: 240, min: 220, sort: "text" },
  { id: "restored", label: "Restored", width: 140, min: 100, numeric: true, sort: "number" },
  { id: "started", label: "Started", width: 120, min: 110, numeric: true, sort: "time" },
];

export type MaintenanceColumn =
  | "maintenance"
  | "repository"
  | "quickLast"
  | "quickNext"
  | "quickFailures"
  | "fullLast"
  | "fullNext"
  | "fullFailures"
  | "reclaimed"
  | "manual"
  | "action";

export const MAINTENANCE_COLUMNS: readonly ColumnSpec<MaintenanceColumn>[] = [
  {
    id: "maintenance",
    label: "Maintenance",
    width: 240,
    min: 220,
    locked: true,
    stripe: true,
    sort: "text",
  },
  { id: "repository", label: "Repository", width: 220, min: 210, sort: "text" },
  { id: "quickLast", label: "Quick last run", width: 140, min: 140, sort: "time" },
  {
    id: "quickNext",
    label: "Quick next run",
    width: 140,
    min: 140,
    defaultHidden: true,
    sort: "time",
  },
  { id: "quickFailures", label: "Quick failures", width: 130, min: 130, sort: "number" },
  { id: "fullLast", label: "Full last run", width: 130, min: 130, sort: "time" },
  {
    id: "fullNext",
    label: "Full next run",
    width: 140,
    min: 130,
    defaultHidden: true,
    sort: "time",
  },
  { id: "fullFailures", label: "Full failures", width: 120, min: 120, sort: "number" },
  { id: "reclaimed", label: "Reclaimed", width: 110, min: 100, numeric: true, sort: "number" },
  { id: "manual", label: "Manual run", width: 200, min: 120 },
];

/** With the route's run control: a column of buttons, kept last. */
export const MAINTENANCE_COLUMNS_WITH_ACTION: readonly ColumnSpec<MaintenanceColumn>[] = [
  ...MAINTENANCE_COLUMNS,
  { id: "action", label: "Run", width: 110, min: 110, locked: true, resizable: false },
];

export type ReplicationColumn =
  | "replication"
  | "state"
  | "copies"
  | "schedule"
  | "lastReplicated"
  | "lastRun"
  | "run";

export const REPLICATION_COLUMNS: readonly ColumnSpec<ReplicationColumn>[] = [
  {
    id: "replication",
    label: "Replication",
    width: 260,
    min: 240,
    locked: true,
    stripe: true,
    sort: "text",
  },
  { id: "state", label: "State", width: 140, min: 130, sort: "state" },
  { id: "copies", label: "Copies", width: 260, min: 140 },
  { id: "schedule", label: "Schedule", width: 170, min: 110 },
  { id: "lastReplicated", label: "Last replicated", width: 160, min: 150, sort: "time" },
  { id: "lastRun", label: "Last run", width: 160, min: 100 },
  { id: "run", label: "Run", width: 150, min: 120, locked: true, resizable: false },
];

export type DoctorColumn = "outcome" | "check" | "scope" | "finding";

/**
 * The outcome and the check name the row, so neither can be hidden or moved;
 * the finding is the prose column and takes the room that is left.
 */
export const DOCTOR_COLUMNS: readonly ColumnSpec<DoctorColumn>[] = [
  { id: "outcome", label: "Outcome", width: 130, min: 130, locked: true, sort: "state" },
  { id: "check", label: "Check", width: 240, min: 140, locked: true, sort: "text" },
  {
    id: "scope",
    label: "Scope",
    width: 200,
    min: 160,
    className: "doctor-checks__scope",
    sort: "text",
  },
  { id: "finding", label: "Finding", width: 400, min: 260 },
];

export type GateColumn = "severity" | "condition" | "blockedWhen" | "reason" | "appliesTo";

/** Severity and condition name the gate, so they stay; the reason takes what is left. */
export const GATE_COLUMNS: readonly ColumnSpec<GateColumn>[] = [
  { id: "severity", label: "Severity", width: 130, min: 130, locked: true, sort: "state" },
  {
    id: "condition",
    label: "Condition",
    width: 240,
    min: 120,
    locked: true,
    className: "mono",
    sort: "text",
  },
  {
    id: "blockedWhen",
    label: "Blocked when",
    width: 160,
    min: 130,
    className: "mono gate-list__status",
  },
  { id: "reason", label: "Reason", width: 300, min: 180, className: "mono", sort: "text" },
  { id: "appliesTo", label: "Applies to", width: 260, min: 150 },
];

export type SnapshotColumn =
  | "snapshot"
  | "phase"
  | "origin"
  | "policy"
  | "repository"
  | "size"
  | "files"
  | "started"
  | "took";

export const SNAPSHOT_COLUMNS: readonly ColumnSpec<SnapshotColumn>[] = [
  {
    id: "snapshot",
    label: "Snapshot",
    width: 240,
    min: 200,
    locked: true,
    stripe: true,
    sort: "text",
  },
  { id: "phase", label: "Phase", width: 130, min: 130, sort: "state" },
  { id: "origin", label: "Origin", width: 110, min: 80, sort: "text" },
  { id: "policy", label: "Policy", width: 210, min: 200, sort: "text" },
  {
    id: "repository",
    label: "Repository",
    width: 220,
    min: 210,
    className: "snapshot-table__repository",
    sort: "text",
  },
  { id: "size", label: "Size", width: 100, min: 96, numeric: true, sort: "number" },
  { id: "files", label: "Files", width: 110, min: 80, numeric: true, sort: "number" },
  { id: "started", label: "Started", width: 100, min: 90, numeric: true, sort: "time" },
  { id: "took", label: "Took", width: 80, min: 70, numeric: true, sort: "number" },
];
