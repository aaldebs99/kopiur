import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { formatTimestamp } from "../util/format";
import { RowTime } from "./RowTime";

describe("RowTime", () => {
  it("says how long ago, and the absolute instant on hover", () => {
    const at = "2026-09-08T11:55:00Z";
    render(<RowTime at={at} now={new Date("2026-09-08T12:00:00Z")} />);
    const time = screen.getByText("5m ago");
    expect(time.tagName).toBe("TIME");
    expect(time).toHaveAttribute("datetime", at);
    expect(time).toHaveAttribute("title", formatTimestamp(at));
    expect(time).toHaveClass("attention-row__at");
  });
});
