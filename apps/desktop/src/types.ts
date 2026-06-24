export type ImageFormat = "jpeg" | "png" | "webp" | "heic" | "bmp" | "gif";
export type CandidateStatus =
  | "found"
  | "partial"
  | "recovered"
  | "failed"
  | "corrupted";
export type ScanMode = "normal" | "deep";

export interface VolumeInfo {
  id: string;
  rootPath: string;
  devicePath: string;
  displayName: string;
  totalBytes: number;
  sectorSize: number;
  isRemovable: boolean;
}

export interface RecoveryCandidate {
  id: string;
  scanId: string;
  format: ImageFormat;
  offset: number;
  estimatedSize: number;
  confidence: number;
  status: CandidateStatus;
  possibleOriginalName: string | null;
  recoveredPath: string | null;
}

export interface ScanProgress {
  scanId: string;
  bytesScanned: number;
  totalBytes: number;
  foundCount: number;
  completeCount: number;
  partialCount: number;
  corruptedCount: number;
  readErrorCount: number;
  scanMode: ScanMode;
  status: "starting" | "scanning" | "completed" | "cancelled" | "failed";
  message: string;
}

export interface RecoveryProgress {
  scanId: string;
  completed: number;
  total: number;
  candidateId: string | null;
  success: boolean;
  message: string;
}

export interface StartScanResponse {
  scanId: string;
  sessionDirectory: string;
}

export interface PreviewData {
  dataUrl: string;
}
