import type { ReactNode } from "react";

import type { GateDescriptor } from "../api/types";
import { ColumnLedger } from "./ColumnLedger";
import { HealthBadge } from "./HealthBadge";
import { gateScopeKinds, gateSeverityLamp, sortGates } from "./gates";
import type { SortValue } from "./tableColumns";
import { GATE_COLUMNS, type GateColumn } from "./listColumns";
import { healthRank } from "./health";

/**
 * The structural-gate registry as a ledger: severity lamp, the condition
 * that carries the gate, the status value that means blocked (per gate —
 * some block on `False`, some on `True`), the reason the reconciler writes,
 * and the kinds the condition appears on.
 *
 * Rendered from `GateDescriptor` alone, so a gate a newer operator raises
 * is explained in the operator's own words with no lookup table here.
 */
export interface GateListProps {
  gates: readonly GateDescriptor[];
}

export function GateList({ gates }: GateListProps) {
  return (
    <ColumnLedger
      id="gates"
      label="Structural gates"
      className="gate-list"
      columns={GATE_COLUMNS}
      rows={sortGates(gates)}
      rowKey={(gate) => `${gate.condition}/${gate.reason}`}
      cell={gateCell}
      sortValue={gateSortValue}
    />
  );
}

function gateSortValue(gate: GateDescriptor, id: GateColumn): SortValue {
  switch (id) {
    case "severity":
      return healthRank(gateSeverityLamp(gate.severity).key);
    case "condition":
      return gate.condition;
    case "reason":
      return gate.reason;
    case "blockedWhen":
    case "appliesTo":
      return null;
    default:
      return id satisfies never;
  }
}

function gateCell(gate: GateDescriptor, id: GateColumn): ReactNode {
  switch (id) {
    case "severity": {
      const lamp = gateSeverityLamp(gate.severity);
      return <HealthBadge health={lamp.key} label={lamp.word} />;
    }
    case "condition":
      return gate.condition;
    case "blockedWhen":
      return (
        <>
          <span className="gate-list__status-eq">status =</span> {gate.blockedStatus}
        </>
      );
    case "reason":
      return gate.reason;
    case "appliesTo":
      return (
        <ul className="gate-list__kinds" aria-label="Kinds">
          {gateScopeKinds(gate.scope).map((kind) => (
            <li key={kind} className="label-strip">
              <span className="label-strip__kind">{kind}</span>
            </li>
          ))}
        </ul>
      );
    default:
      return id satisfies never;
  }
}
