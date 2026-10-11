import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { useEffect, useLayoutEffect, useRef } from "react";

import { commitWidths, resetWidth, setWidth } from "./columnPrefs";
import {
  COLUMN_STEP,
  type ColumnSpec,
  MAX_COLUMN_WIDTH,
  type SortKind,
  type TableSort,
  clampWidth,
  nextSort,
} from "./tableColumns";

/**
 * One column's header, with the handle on its right edge that resizes it.
 *
 * The handle is a focusable separator like the drawer's grip and the
 * overview's split: drag it, or focus it and use the arrow keys; Home or a
 * double click puts the declared width back. A drag is measured from where
 * it started (the width then plus how far the pointer has gone), so a column
 * never creeps; the width is stored when the drag ends, and on each key.
 *
 * The last column has no handle: its right edge is the table's, and it takes
 * whatever room the others leave.
 *
 * A sortable column's words are a button that changes the table's order; the
 * cell says how it is ordered now (`aria-sort`), and the button's title says
 * what the next click does. The handle is a sibling, never inside it, so a
 * drag never sorts.
 */
export function ColumnHeader({
  table,
  spec,
  width,
  last = false,
  sort,
  onSort,
}: {
  table: string;
  spec: ColumnSpec;
  /** What the table draws it at. */
  width: number;
  /** The last column on screen, which takes the room left and has no handle. */
  last?: boolean;
  /**
   * The table's order as it touches this column: the sort when it is this
   * column, `null` when it is another (or none), `undefined` when the table
   * does not sort at all.
   */
  sort?: TableSort | null | undefined;
  onSort?: (() => void) | undefined;
}) {
  const header = useRef<HTMLTableCellElement | null>(null);
  const handle = useRef<HTMLDivElement | null>(null);
  const widthRef = useRef(width);
  useLayoutEffect(() => {
    widthRef.current = width;
  }, [width]);
  const resizable = spec.resizable !== false && !last;

  useEffect(() => {
    const node = handle.current;
    if (node === null) return;
    // A focusable separator is a widget the a11y lint's role table does not
    // yet know; the tab stop is set here, as on the drawer's grip.
    node.tabIndex = 0;
    const resize = (px: number) => {
      setWidth(table, spec.id, clampWidth(spec, px));
    };
    // Where a change starts from is the width on screen now — the window or
    // the sidebar may have moved it since anything was measured — falling
    // back to what the table asked for where nothing is laid out.
    const current = () => {
      const measured = Math.round(header.current?.getBoundingClientRect().width ?? 0);
      return measured > 0 ? measured : widthRef.current;
    };
    let start: { x: number; width: number } | null = null;
    const onDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      event.preventDefault();
      event.stopPropagation();
      start = { x: event.clientX, width: current() };
      node.setPointerCapture(event.pointerId);
      node.dataset.dragging = "true";
    };
    const onMove = (event: PointerEvent) => {
      if (start === null) return;
      resize(start.width + (event.clientX - start.x));
    };
    const onUp = (event: PointerEvent) => {
      if (start === null) return;
      start = null;
      node.releasePointerCapture(event.pointerId);
      delete node.dataset.dragging;
      commitWidths();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Home") {
        event.preventDefault();
        resetWidth(table, spec.id);
        return;
      }
      const step =
        event.key === "ArrowRight" ? COLUMN_STEP : event.key === "ArrowLeft" ? -COLUMN_STEP : 0;
      if (step === 0) return;
      event.preventDefault();
      resize(current() + step);
      commitWidths();
    };
    const onDouble = () => {
      resetWidth(table, spec.id);
    };
    node.addEventListener("pointerdown", onDown);
    node.addEventListener("pointermove", onMove);
    node.addEventListener("pointerup", onUp);
    node.addEventListener("pointercancel", onUp);
    node.addEventListener("keydown", onKey);
    node.addEventListener("dblclick", onDouble);
    return () => {
      node.removeEventListener("pointerdown", onDown);
      node.removeEventListener("pointermove", onMove);
      node.removeEventListener("pointerup", onUp);
      node.removeEventListener("pointercancel", onUp);
      node.removeEventListener("keydown", onKey);
      node.removeEventListener("dblclick", onDouble);
    };
  }, [table, spec, resizable]);

  const kind = sort === undefined ? undefined : spec.sort;
  const label = (
    <span className="ledger__label" title={kind === undefined ? spec.label : undefined}>
      {spec.label}
    </span>
  );
  return (
    <th
      ref={header}
      scope="col"
      className={spec.numeric === true ? "num" : undefined}
      aria-sort={
        kind === undefined
          ? undefined
          : sort === null || sort === undefined
            ? "none"
            : sort.descending
              ? "descending"
              : "ascending"
      }
    >
      {kind === undefined ? (
        label
      ) : (
        <button
          type="button"
          className="ledger__sort"
          aria-label={`Sort by ${spec.label}`}
          title={sortHint(kind, nextSort(sort ?? null, spec))}
          data-sorted={sort === null || sort === undefined ? undefined : "true"}
          onClick={onSort}
        >
          {label}
          <SortIcon sort={sort ?? null} />
        </button>
      )}
      {resizable ? (
        <div
          ref={handle}
          className="ledger__resize"
          role="separator"
          aria-orientation="vertical"
          aria-label={`Resize ${spec.label}`}
          aria-valuemin={spec.min}
          aria-valuemax={MAX_COLUMN_WIDTH}
          aria-valuenow={width}
          aria-valuetext={`${String(width)} pixels`}
        />
      ) : null}
    </th>
  );
}

function SortIcon({ sort }: { sort: TableSort | null }) {
  const Icon = sort === null ? ArrowUpDown : sort.descending ? ArrowDown : ArrowUp;
  return <Icon className="ledger__sort-icon" size={12} strokeWidth={2} aria-hidden="true" />;
}

/** What the next click does, in the column's own terms. */
function sortHint(kind: SortKind, next: TableSort | null): string {
  if (next === null) return "Clear the sort";
  return `Sort ${directionWords(kind, next.descending)}`;
}

/** Which way a column runs, read the way its values are. */
function directionWords(kind: SortKind, descending: boolean): string {
  switch (kind) {
    case "text":
      return descending ? "Z to A" : "A to Z";
    case "number":
      return descending ? "largest first" : "smallest first";
    case "time":
      return descending ? "newest first" : "oldest first";
    case "state":
      return descending ? "healthiest first" : "worst first";
    default:
      return kind satisfies never;
  }
}
