export type FoundationStatus = {
  product: string;
  status: string;
  platform: string;
  scanningImplemented: boolean;
  deletionImplemented: boolean;
};

export const fallbackFoundation: FoundationStatus = {
  product: "DiskClearance",
  status: "Pre-implementation foundation",
  platform: "browser preview",
  scanningImplemented: false,
  deletionImplemented: false,
};
