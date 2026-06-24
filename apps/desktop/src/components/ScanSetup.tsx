import type { ScanMode, VolumeInfo } from "../types";
import { formatBytes } from "../lib/format";

interface Props {
  volumes: VolumeInfo[];
  selectedVolume: string;
  scanMode: ScanMode;
  workspaceDirectory: string;
  busy: boolean;
  onVolumeChange: (id: string) => void;
  onScanModeChange: (mode: ScanMode) => void;
  onChooseWorkspace: () => void;
  onStart: () => void;
}

export function ScanSetup({
  volumes,
  selectedVolume,
  scanMode,
  workspaceDirectory,
  busy,
  onVolumeChange,
  onScanModeChange,
  onChooseWorkspace,
  onStart,
}: Props) {
  return (
    <section className="panel setup-panel">
      <div className="section-heading">
        <div>
          <span className="eyebrow">Etapa 1</span>
          <h2>Escolha a unidade de origem</h2>
        </div>
        <span className="read-only-badge">Somente leitura</span>
      </div>

      <div className="volume-grid">
        {volumes.map((volume) => (
          <button
            key={volume.id}
            className={`volume-card ${
              selectedVolume === volume.id ? "selected" : ""
            }`}
            onClick={() => onVolumeChange(volume.id)}
            disabled={busy}
          >
            <span className="drive-icon" aria-hidden="true" />
            <span>
              <strong>{volume.displayName}</strong>
              <small>
                {formatBytes(volume.totalBytes)} -{" "}
                {volume.isRemovable ? "Removivel" : "Unidade fixa"}
              </small>
            </span>
            <span className="radio-dot" />
          </button>
        ))}
      </div>

      <div className="scan-mode-row">
        <div>
          <label>Modo de varredura</label>
          <p>
            {scanMode === "deep"
              ? "A varredura profunda procura imagens parciais e pode demorar mais."
              : "A varredura normal prioriza candidatos completos e mais rapidos."}
          </p>
        </div>
        <div className="mode-toggle" role="group" aria-label="Modo de varredura">
          <button
            type="button"
            className={scanMode === "normal" ? "active" : ""}
            onClick={() => onScanModeChange("normal")}
            disabled={busy}
          >
            Varredura normal
          </button>
          <button
            type="button"
            className={scanMode === "deep" ? "active" : ""}
            onClick={() => onScanModeChange("deep")}
            disabled={busy}
          >
            Varredura profunda
          </button>
        </div>
      </div>

      <div className="workspace-row">
        <div>
          <label>Pasta segura de trabalho e recuperação</label>
          <p>
            O índice, os logs e as fotos recuperadas serão gravados aqui.
          </p>
        </div>
        <button
          className="path-picker"
          onClick={onChooseWorkspace}
          disabled={busy}
        >
          <span>{workspaceDirectory || "Escolher pasta em outra unidade"}</span>
          <b>Selecionar</b>
        </button>
      </div>

      <button
        className="primary start-button"
        onClick={onStart}
        disabled={busy || !selectedVolume || !workspaceDirectory}
      >
        {scanMode === "deep"
          ? "Iniciar varredura profunda"
          : "Iniciar varredura normal"}
      </button>
    </section>
  );
}
