// Domain types shared by the store and the pages.
//
// They mirror the sidecar's contract (`contract/contract.json`) so the pages
// already speak the shape the backend will report. P1 replaces this file with
// the Rust model's generated declarations; until then it keeps the two sides
// honest about names.

/** Where updates come from. */
export type UpdateSource = "github" | "mirrorChyan";

/** The update mirror's label, as the settings page shows it. */
export const UPDATE_SOURCE_LABELS: Record<UpdateSource, string> = {
  github: "GitHub",
  mirrorChyan: "Mirror 酱",
};
