import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { ActivityLog } from "./components/ActivityLog";
import { CandidateTable } from "./components/CandidateTable";
import { RecoveryBar } from "./components/RecoveryBar";
import { SafetyNotice } from "./components/SafetyNotice";
import { ScanSetup } from "./components/ScanSetup";
import { ScanStatus } from "./components/ScanStatus";
import type {
  RecoveryCandidate,
  RecoveryProgress,
  ScanMode,
  ScanProgress,
  VolumeInfo,
} from "./types";

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Ocorreu um erro inesperado.";
}

export default function App() {
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [selectedVolume, setSelectedVolume] = useState("");
  const [scanMode, setScanMode] = useState<ScanMode>("normal");
  const [workspaceDirectory, setWorkspaceDirectory] = useState("");
  const [destination, setDestination] = useState("");
  const [scanId, setScanId] = useState<string | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [candidates, setCandidates] = useState<RecoveryCandidate[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [previewCandidateId, setPreviewCandidateId] = useState<string | null>(
    null,
  );
  const [elevated, setElevated] = useState(false);
  const [recovering, setRecovering] = useState(false);
  const [logs, setLogs] = useState<string[]>([
    "Aguardando a seleção de uma unidade e de uma pasta segura.",
  ]);

  const scanning =
    progress?.status === "starting" || progress?.status === "scanning";
  const busy = scanning || recovering;

  const selectedCandidates = useMemo(
    () => candidates.filter((candidate) => selected.has(candidate.id)),
    [candidates, selected],
  );

  useEffect(() => {
    Promise.all([api.listVolumes(), api.isElevated()])
      .then(([availableVolumes, admin]) => {
        setVolumes(availableVolumes);
        setElevated(admin);
        if (availableVolumes[0]) setSelectedVolume(availableVolumes[0].id);
      })
      .catch((error) => setLogs((current) => [...current, errorMessage(error)]));
  }, []);

  useEffect(() => {
    const unsubscribers = Promise.all([
      listen<ScanProgress>("scan-progress", (event) => {
        setProgress(event.payload);
        setLogs((current) =>
          current.at(-1) === event.payload.message
            ? current
            : [...current, event.payload.message],
        );
        if (
          event.payload.status === "completed" ||
          event.payload.status === "cancelled"
        ) {
          api
            .listCandidates(event.payload.scanId)
            .then(setCandidates)
            .catch((error) =>
              setLogs((current) => [...current, errorMessage(error)]),
            );
        }
      }),
      listen<RecoveryCandidate>("candidate-found", (event) => {
        setCandidates((current) =>
          current.some((candidate) => candidate.id === event.payload.id)
            ? current
            : [...current, event.payload],
        );
      }),
      listen<RecoveryProgress>("recovery-progress", (event) => {
        setLogs((current) => [...current, event.payload.message]);
        if (event.payload.completed === event.payload.total) {
          setRecovering(false);
          if (scanId) {
            api.listCandidates(scanId).then(setCandidates).catch(() => undefined);
          }
        }
      }),
    ]);

    return () => {
      unsubscribers.then((items) => items.forEach((unsubscribe) => unsubscribe()));
    };
  }, [scanId]);

  async function chooseFolder(setter: (path: string) => void) {
    const selectedPath = await open({ directory: true, multiple: false });
    if (typeof selectedPath === "string") setter(selectedPath);
  }

  async function startScan() {
    try {
      setCandidates([]);
      setSelected(new Set());
      setPreviewUrl(null);
      setProgress({
        scanId: "",
        bytesScanned: 0,
        totalBytes:
          volumes.find((volume) => volume.id === selectedVolume)?.totalBytes ?? 0,
        foundCount: 0,
        completeCount: 0,
        partialCount: 0,
        corruptedCount: 0,
        readErrorCount: 0,
        scanMode,
        status: "starting",
        message: "Preparando uma sessão segura em outra unidade…",
      });
      const response = await api.startScan(
        selectedVolume,
        workspaceDirectory,
        scanMode,
      );
      setScanId(response.scanId);
      setDestination(workspaceDirectory);
      setLogs((current) => [
        ...current,
        `Sessão criada em ${response.sessionDirectory}.`,
      ]);
    } catch (error) {
      setProgress(null);
      setLogs((current) => [...current, errorMessage(error)]);
    }
  }

  async function cancelScan() {
    if (!scanId) return;
    await api.cancelScan(scanId);
  }

  function toggleCandidate(id: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function toggleAll() {
    setSelected((current) =>
      current.size === candidates.length
        ? new Set()
        : new Set(candidates.map((candidate) => candidate.id)),
    );
  }

  async function preview(candidate: RecoveryCandidate) {
    if (!scanId) return;
    try {
      const data = await api.previewCandidate(scanId, candidate.id);
      setPreviewUrl(data.dataUrl);
      setPreviewCandidateId(candidate.id);
    } catch (error) {
      setLogs((current) => [...current, errorMessage(error)]);
    }
  }

  async function recover() {
    if (!scanId || selectedCandidates.length === 0) return;
    try {
      setRecovering(true);
      await api.recoverCandidates(
        scanId,
        selectedCandidates.map((candidate) => candidate.id),
        destination,
      );
    } catch (error) {
      setRecovering(false);
      setLogs((current) => [...current, errorMessage(error)]);
    }
  }

  async function elevate() {
    try {
      await api.restartElevated();
      setLogs((current) => [
        ...current,
        "A solicitação do Windows foi aberta. Esta janela pode ser fechada.",
      ]);
    } catch (error) {
      setLogs((current) => [...current, errorMessage(error)]);
    }
  }

  return (
    <main>
      <header className="app-header">
        <div className="brand-mark" aria-hidden="true">
          <span />
        </div>
        <div>
          <h1>PhotoRescue</h1>
          <p>Recuperação de imagens com leitura bruta e segura</p>
        </div>
        <div className={`admin-state ${elevated ? "ok" : "warning"}`}>
          {elevated ? "Administrador ativo" : "Permissão limitada"}
        </div>
      </header>

      <SafetyNotice elevated={elevated} onElevate={elevate} />
      <ScanSetup
        volumes={volumes}
        selectedVolume={selectedVolume}
        scanMode={scanMode}
        workspaceDirectory={workspaceDirectory}
        busy={busy}
        onVolumeChange={setSelectedVolume}
        onScanModeChange={setScanMode}
        onChooseWorkspace={() => chooseFolder(setWorkspaceDirectory)}
        onStart={startScan}
      />
      <ScanStatus progress={progress} onCancel={cancelScan} />
      <CandidateTable
        candidates={candidates}
        selected={selected}
        previewUrl={previewUrl}
        previewCandidateId={previewCandidateId}
        onToggle={toggleCandidate}
        onToggleAll={toggleAll}
        onPreview={preview}
      />
      <RecoveryBar
        selectedCount={selected.size}
        destination={destination}
        busy={recovering}
        onChooseDestination={() => chooseFolder(setDestination)}
        onRecover={recover}
      />
      <ActivityLog messages={logs} />

      <footer>
        PhotoRescue MVP 1 · A recuperação não é garantida · Nunca salve na
        unidade de origem
      </footer>
    </main>
  );
}
