import { describe, expect, it } from "vitest";

import {
  DOCTOR_COLUMNS,
  GATE_COLUMNS,
  MAINTENANCE_COLUMNS_WITH_ACTION,
  POLICY_COLUMNS,
  REPLICATION_COLUMNS,
  REPOSITORY_COLUMNS,
  RESTORE_COLUMNS,
  SCHEDULE_COLUMNS_WITH_ACTION,
  SNAPSHOT_COLUMNS,
} from "./listColumns";
import type { ColumnSpec } from "./tableColumns";

/**
 * Which columns sort, table by table. Only where an order means something: a
 * list of references, prose, a cron line or a button never sorts.
 */
const EXPECTED: [string, readonly ColumnSpec[], Record<string, string>][] = [
  [
    "repositories",
    REPOSITORY_COLUMNS,
    {
      repository: "text",
      health: "state",
      phase: "text",
      backend: "text",
      snapshots: "number",
      size: "number",
      indexBlobs: "number",
      lastObserved: "time",
    },
  ],
  [
    "policies",
    POLICY_COLUMNS,
    {
      policy: "text",
      state: "state",
      lastSnapshot: "time",
      lastVerified: "time",
      snapshots: "number",
    },
  ],
  [
    "schedules",
    SCHEDULE_COLUMNS_WITH_ACTION,
    { schedule: "text", state: "state", lastFire: "time", nextFire: "time" },
  ],
  [
    "restores",
    RESTORE_COLUMNS,
    { restore: "text", phase: "state", repository: "text", restored: "number", started: "time" },
  ],
  [
    "maintenance",
    MAINTENANCE_COLUMNS_WITH_ACTION,
    {
      maintenance: "text",
      repository: "text",
      quickLast: "time",
      quickNext: "time",
      quickFailures: "number",
      fullLast: "time",
      fullNext: "time",
      fullFailures: "number",
      reclaimed: "number",
    },
  ],
  [
    "replications",
    REPLICATION_COLUMNS,
    { replication: "text", state: "state", lastReplicated: "time" },
  ],
  ["doctor", DOCTOR_COLUMNS, { outcome: "state", check: "text", scope: "text" }],
  ["gates", GATE_COLUMNS, { severity: "state", condition: "text", reason: "text" }],
  [
    "snapshots",
    SNAPSHOT_COLUMNS,
    {
      snapshot: "text",
      phase: "state",
      origin: "text",
      policy: "text",
      repository: "text",
      size: "number",
      files: "number",
      started: "time",
      took: "number",
    },
  ],
];

describe("sortable columns", () => {
  it.each(EXPECTED)(
    "%s sorts exactly the columns an order means something for",
    (_, columns, sorts) => {
      const declared = Object.fromEntries(
        columns.filter((c) => c.sort !== undefined).map((c) => [c.id, c.sort]),
      );
      expect(declared).toEqual(sorts);
    },
  );

  it("never sorts a column of buttons", () => {
    for (const [, columns] of EXPECTED) {
      for (const column of columns.filter((c) => c.resizable === false)) {
        expect(column.sort, column.id).toBeUndefined();
      }
    }
  });
});
