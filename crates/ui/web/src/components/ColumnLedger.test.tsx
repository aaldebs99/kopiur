import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRouter,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { bodyRows, nth } from "../test-utils";
import { ColumnLedger } from "./ColumnLedger";
import { COLUMNS_KEY, forgetColumnPrefs, setHidden } from "./columnPrefs";
import type { ColumnSpec } from "./tableColumns";

type Id = "name" | "phase" | "size" | "run";

const COLUMNS: readonly ColumnSpec<Id>[] = [
  { id: "name", label: "Snapshot", width: 260, min: 200, locked: true, stripe: true },
  { id: "phase", label: "Phase", width: 130, min: 90 },
  { id: "size", label: "Size", width: 100, min: 70, numeric: true },
  { id: "run", label: "Run", width: 90, min: 90, locked: true, resizable: false },
];

interface Row {
  name: string;
  phase: string;
  size: string;
}

const ROWS: Row[] = [
  { name: "photos-1", phase: "Succeeded", size: "2 KiB" },
  { name: "photos-2", phase: "Failed", size: "—" },
];

/** Sizes as numbers, the empty cell as none. */
function sortValue(row: Row, id: Id) {
  switch (id) {
    case "name":
      return row.name;
    case "phase":
      return row.phase;
    case "size":
      return row.size === "—" ? null : Number.parseFloat(row.size);
    case "run":
      return null;
  }
}

function cell(row: Row, id: Id) {
  switch (id) {
    case "name":
      return row.name;
    case "phase":
      return row.phase;
    case "size":
      return row.size;
    case "run":
      return <button type="button">Run</button>;
  }
}

interface MountOptions {
  columns?: readonly ColumnSpec<Id>[];
  path?: string;
  serverSorted?: boolean;
}

async function mount(columns: readonly ColumnSpec<Id>[] = COLUMNS, options: MountOptions = {}) {
  const ledger = (
    <ColumnLedger
      id="test"
      label="Test table"
      className="test-table"
      columns={columns}
      rows={ROWS}
      rowKey={(r) => r.name}
      rowProps={() => ({ "data-kind": "snapshot" })}
      cell={cell}
      sortValue={sortValue}
      serverSorted={options.serverSorted}
    />
  );
  const root = createRootRoute({ component: () => ledger });
  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: [options.path ?? "/"] }),
  });
  render(<RouterProvider router={router} />);
  const table = await screen.findByRole("table", { name: "Test table" });
  return Object.assign(table, { router });
}

const stored = () => JSON.parse(window.localStorage.getItem(COLUMNS_KEY) ?? "{}") as unknown;
const headers = (table: HTMLElement) =>
  within(table)
    .getAllByRole("columnheader")
    .map((th) => th.querySelector(".ledger__label")?.textContent);

beforeEach(() => {
  window.localStorage.clear();
  forgetColumnPrefs();
});

describe("ColumnLedger", () => {
  it("draws the declared columns in order, the first with the kind stripe", async () => {
    const table = await mount();
    expect(table).toHaveClass("ledger", "ledger--fixed", "test-table");
    expect(headers(table)).toEqual(["Snapshot", "Phase", "Size", "Run"]);
    const row = nth(bodyRows(table), 0);
    expect(row).toHaveAttribute("data-kind", "snapshot");
    const cells = Array.from(row.querySelectorAll("td"));
    expect(cells.map((td) => td.textContent)).toEqual(["photos-1", "Succeeded", "2 KiB", "Run"]);
    expect(nth(cells, 0)).toHaveClass("has-stripe");
    expect(nth(cells, 2)).toHaveClass("num");
  });

  it("offers its own Columns menu, above the card", async () => {
    const table = await mount();
    const menu = screen.getByRole("button", { name: "Columns" });
    expect(menu.compareDocumentPosition(table) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("sizes every column through a colgroup but the last, which takes the room left", async () => {
    const table = await mount();
    const cols = Array.from(table.querySelectorAll<HTMLElement>("colgroup col"));
    expect(cols.map((c) => c.style.width)).toEqual(["260px", "130px", "100px", ""]);
    // Every column at its width, the last at its floor: 260 + 130 + 100 + 90.
    expect(table.style.minWidth).toBe("580px");
    expect(table.style.width).toBe("100%");
  });

  it("has no handle on the last column, whose right edge is the table's", async () => {
    await mount(COLUMNS.slice(0, 3));
    expect(screen.getByRole("separator", { name: "Resize Phase" })).toBeInTheDocument();
    expect(screen.queryByRole("separator", { name: "Resize Size" })).toBeNull();
  });

  it("changes only the column being resized", async () => {
    const table = await mount();
    const widths = () =>
      Array.from(table.querySelectorAll<HTMLElement>("colgroup col")).map((c) => c.style.width);
    const user = userEvent.setup();
    screen.getByRole("separator", { name: "Resize Phase" }).focus();
    await user.keyboard("{ArrowRight}{ArrowRight}{ArrowRight}");
    expect(widths()).toEqual(["260px", "178px", "100px", ""]);
    screen.getByRole("separator", { name: "Resize Snapshot" }).focus();
    await user.keyboard("{ArrowLeft}");
    expect(widths()).toEqual(["244px", "178px", "100px", ""]);
  });

  it("drops a hidden column from the header and every row together", async () => {
    const table = await mount();
    act(() => {
      setHidden("test", "phase", true);
    });
    expect(headers(table)).toEqual(["Snapshot", "Size", "Run"]);
    for (const row of bodyRows(table)) {
      expect(row.querySelectorAll("td")).toHaveLength(3);
    }
    expect(table.querySelectorAll("colgroup col")).toHaveLength(3);
  });

  it("gives each resizable column a handle named for it, in pixels", async () => {
    await mount();
    const handle = screen.getByRole("separator", { name: "Resize Phase" });
    expect(handle).toHaveAttribute("aria-orientation", "vertical");
    expect(handle).toHaveAttribute("aria-valuenow", "130");
    expect(handle).toHaveAttribute("aria-valuemin", "90");
    expect(handle).toHaveAttribute("aria-valuetext", "130 pixels");
    expect(screen.queryByRole("separator", { name: "Resize Run" })).toBeNull();
  });

  it("resizes with the arrow keys and remembers it", async () => {
    const table = await mount();
    const handle = screen.getByRole("separator", { name: "Resize Phase" });
    const user = userEvent.setup();
    handle.focus();
    expect(handle).toHaveFocus();
    await user.keyboard("{ArrowRight}{ArrowRight}");
    expect(handle).toHaveAttribute("aria-valuenow", "162");
    expect(stored()).toEqual({ test: { width: { phase: 162 } } });
    expect(nth(Array.from(table.querySelectorAll<HTMLElement>("col")), 1).style.width).toBe(
      "162px",
    );
    await user.keyboard("{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}");
    expect(handle).toHaveAttribute("aria-valuenow", "90");
  });

  it("goes back to its declared width on Home or a double click", async () => {
    await mount();
    const handle = screen.getByRole("separator", { name: "Resize Size" });
    const user = userEvent.setup();
    handle.focus();
    await user.keyboard("{ArrowRight}");
    expect(handle).toHaveAttribute("aria-valuenow", "116");
    await user.keyboard("{Home}");
    expect(handle).toHaveAttribute("aria-valuenow", "100");
    await user.keyboard("{ArrowRight}");
    await user.dblClick(handle);
    expect(handle).toHaveAttribute("aria-valuenow", "100");
    expect(stored()).toEqual({});
  });

  it("follows a drag from where it started, and stores it once the drag ends", async () => {
    await mount();
    const handle = screen.getByRole("separator", { name: "Resize Phase" });
    handle.setPointerCapture = () => undefined;
    handle.releasePointerCapture = () => undefined;
    fireEvent.pointerDown(handle, { button: 0, clientX: 300, pointerId: 1 });
    fireEvent.pointerMove(handle, { clientX: 340, pointerId: 1 });
    fireEvent.pointerMove(handle, { clientX: 360, pointerId: 1 });
    expect(handle).toHaveAttribute("aria-valuenow", "190");
    expect(stored()).toEqual({});
    fireEvent.pointerUp(handle, { clientX: 360, pointerId: 1 });
    expect(stored()).toEqual({ test: { width: { phase: 190 } } });
  });

  it("opens with the widths it was left at", async () => {
    window.localStorage.setItem(COLUMNS_KEY, JSON.stringify({ test: { width: { size: 150 } } }));
    forgetColumnPrefs();
    await mount();
    expect(screen.getByRole("separator", { name: "Resize Size" })).toHaveAttribute(
      "aria-valuenow",
      "150",
    );
  });

  describe("sorting", () => {
    const SORTED: readonly ColumnSpec<Id>[] = COLUMNS.map((c) =>
      c.id === "name" ? { ...c, sort: "text" } : c.id === "size" ? { ...c, sort: "number" } : c,
    );
    const names = (table: HTMLElement) =>
      bodyRows(table).map((row) => row.querySelector("td")?.textContent);
    const search = (table: Awaited<ReturnType<typeof mount>>) =>
      table.router.state.location.search as Record<string, unknown>;

    it("offers a sort only on the columns that declare one", async () => {
      const table = await mount(SORTED);
      const sortable = within(table)
        .getAllByRole("columnheader")
        .filter((th) => th.hasAttribute("aria-sort"))
        .map((th) => th.querySelector(".ledger__label")?.textContent);
      expect(sortable).toEqual(["Snapshot", "Size"]);
      expect(within(table).getAllByRole("button", { name: /^Sort by/ })).toHaveLength(2);
      expect(within(table).queryByRole("button", { name: "Sort by Phase" })).toBeNull();
    });

    it("sorts on a click, reverses on the next, and lets go on the third", async () => {
      const table = await mount(SORTED, { path: "/?offset=50" });
      const user = userEvent.setup();
      const size = within(table).getByRole("button", { name: "Sort by Size" });
      await user.click(size);
      expect(search(table).sort).toBe("-size");
      expect(search(table).offset).toBeUndefined();
      expect(size.closest("th")).toHaveAttribute("aria-sort", "descending");
      expect(names(table)).toEqual(["photos-1", "photos-2"]);
      await user.click(size);
      expect(search(table).sort).toBe("size");
      expect(size.closest("th")).toHaveAttribute("aria-sort", "ascending");
      // The empty size stays last whichever way the column runs.
      expect(names(table)).toEqual(["photos-1", "photos-2"]);
      await user.click(size);
      expect(search(table).sort).toBeUndefined();
      expect(size.closest("th")).toHaveAttribute("aria-sort", "none");
    });

    it("opens sorted from the address", async () => {
      const table = await mount(SORTED, { path: "/?sort=-name" });
      expect(names(table)).toEqual(["photos-2", "photos-1"]);
      expect(
        within(table).getByRole("button", { name: "Sort by Snapshot" }).closest("th"),
      ).toHaveAttribute("aria-sort", "descending");
    });

    it("leaves the rows alone when the server did the sorting", async () => {
      const table = await mount(SORTED, { path: "/?sort=-name", serverSorted: true });
      expect(names(table)).toEqual(["photos-1", "photos-2"]);
      expect(
        within(table).getByRole("button", { name: "Sort by Snapshot" }).closest("th"),
      ).toHaveAttribute("aria-sort", "descending");
    });

    it("never sorts when a column is resized", async () => {
      const table = await mount(SORTED);
      const handle = screen.getByRole("separator", { name: "Resize Size" });
      fireEvent.pointerDown(handle, { button: 0, clientX: 100, pointerId: 1 });
      fireEvent.pointerMove(handle, { clientX: 140, pointerId: 1 });
      fireEvent.pointerUp(handle, { clientX: 140, pointerId: 1 });
      fireEvent.click(handle);
      expect(search(table).sort).toBeUndefined();
    });
  });
});
