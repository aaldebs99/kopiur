import { formatTimestamp, relativeTime } from "../util/format";

/**
 * When, in an overview row: how long ago beside the pill, and the absolute
 * local instant on hover. One element for both lists, so they read alike.
 */
export function RowTime({ at, now }: { at: string; now: Date }) {
  return (
    <time className="attention-row__at" dateTime={at} title={formatTimestamp(at)}>
      {relativeTime(at, now)}
    </time>
  );
}
