import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { renderWithRouter } from "../test-utils";
import { formatTimestamp } from "../util/format";
import { RowTime } from "./RowTime";

const AT = "2026-09-08T11:55:00Z";
const NOW = new Date("2026-09-08T12:00:00Z");
const TARGET = { kind: "snapshot", namespace: "media", name: "nightly-1" } as const;

describe("RowTime", () => {
  it("says how long ago, and the absolute instant on hover", async () => {
    renderWithRouter(<RowTime at={AT} now={NOW} target={TARGET} />);
    const time = await screen.findByText("5m ago");
    expect(time.tagName).toBe("TIME");
    expect(time).toHaveAttribute("datetime", AT);
    expect(time).toHaveAttribute("title", formatTimestamp(AT));
    expect(time).toHaveClass("attention-row__at");
  });

  it("opens the row's object, as the rest of the row does", async () => {
    renderWithRouter(<RowTime at={AT} now={NOW} target={TARGET} />, "/?namespace=media");
    const link = (await screen.findByText("5m ago")).closest("a");
    expect(link?.getAttribute("href")).toMatch(/inspect=snapshot%2Fmedia%2Fnightly-1/);
    expect(link?.getAttribute("href")).toMatch(/namespace=media/);
  });

  it("stays out of the tab order: the row's name is already its link", async () => {
    renderWithRouter(<RowTime at={AT} now={NOW} target={TARGET} />);
    const link = (await screen.findByText("5m ago")).closest("a");
    expect(link).toHaveAttribute("tabindex", "-1");
  });
});
