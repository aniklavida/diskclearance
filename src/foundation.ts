import type { FoundationStatus } from "./types/bindings";

export type { FoundationStatus };

export const fallbackFoundation: FoundationStatus = {
  product: "DiskClearance",
  status: "Pre-implementation foundation",
  platform: "browser preview",
  scanningImplemented: false,
  deletionImplemented: false,
};
