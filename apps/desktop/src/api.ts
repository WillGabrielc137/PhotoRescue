import { invoke } from "@tauri-apps/api/core";
import type {
  PreviewData,
  RecoveryCandidate,
  ScanMode,
  StartScanResponse,
  VolumeInfo,
} from "./types";

export const api = {
  listVolumes: () => invoke<VolumeInfo[]>("list_volumes"),
  isElevated: () => invoke<boolean>("is_elevated"),
  restartElevated: () => invoke<void>("restart_elevated"),
  startScan: (
    sourceId: string,
    workspaceDirectory: string,
    scanMode: ScanMode,
  ) =>
    invoke<StartScanResponse>("start_scan", {
      request: { sourceId, workspaceDirectory, scanMode },
    }),
  cancelScan: (scanId: string) => invoke<void>("cancel_scan", { scanId }),
  listCandidates: (scanId: string) =>
    invoke<RecoveryCandidate[]>("list_candidates", { scanId }),
  previewCandidate: (scanId: string, candidateId: string) =>
    invoke<PreviewData>("preview_candidate", { scanId, candidateId }),
  recoverCandidates: (
    scanId: string,
    candidateIds: string[],
    destination: string,
  ) =>
    invoke<void>("recover_candidates", {
      request: { scanId, candidateIds, destination },
    }),
};
