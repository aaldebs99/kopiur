import { formatTimestamp, relativeTime } from "../util/format";
import { InspectLink } from "./InspectLink";
import type { InspectTarget } from "./inspect";

/**
 * When, in an overview row: how long ago beside the pill, and the absolute
 * local instant on hover. One element for both lists, so they read alike.
 *
 * The time sits above the row's stretched link so hovering it can show its
 * title, which also means it takes the click. So it is a link to the row's
 * object as well, and a click on it opens what a click anywhere else on the
 * row opens. It stays out of the tab order: the row's name is already that
 * link for the keyboard.
 */
export function RowTime({ at, now, target }: { at: string; now: Date; target: InspectTarget }) {
  return (
    <InspectLink className="row-time" target={target} tabIndex={-1}>
      <time className="attention-row__at" dateTime={at} title={formatTimestamp(at)}>
        {relativeTime(at, now)}
      </time>
    </InspectLink>
  );
}
