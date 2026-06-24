import { formatBytes, formatPercent } from "../lib/format";
import type { ScanProgress } from "../types";

interface Props {
  progress: ScanProgress | null;
  onCancel: () => void;
}

export function ScanStatus({ progress, onCancel }: Props) {
  if (!progress) return null;
  const percent = formatPercent(progress.bytesScanned, progress.totalBytes);
  const active =
    progress.status === "starting" || progress.status === "scanning";
  const modeLabel =
    progress.scanMode === "deep" ? "Varredura profunda" : "Varredura normal";

  return (
    <section className="panel progress-panel">
      <div className="section-heading">
        <div>
          <span className="eyebrow">Etapa 2</span>
          <h2>{progress.message}</h2>
        </div>
        <div className="progress-heading-meta">
          <span className={`mode-badge ${progress.scanMode}`}>{modeLabel}</span>
          <strong className="progress-value">{percent.toFixed(1)}%</strong>
        </div>
      </div>
      <div
        className="progress-track"
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div className="progress-fill" style={{ width: `${percent}%` }} />
      </div>
      <div className="progress-meta">
        <span>
          {formatBytes(progress.bytesScanned)} de{" "}
          {formatBytes(progress.totalBytes)}
        </span>
        {active && (
          <button className="text-button" onClick={onCancel}>
            Cancelar com segurança
          </button>
        )}
      </div>
      <div className="progress-stats">
        <span>
          <strong>{progress.foundCount}</strong> encontrados
        </span>
        <span>
          <strong>{progress.completeCount}</strong> integros
        </span>
        <span>
          <strong>{progress.partialCount}</strong> parciais
        </span>
        <span>
          <strong>{progress.corruptedCount}</strong> possivelmente corrompidos
        </span>
        <span>
          <strong>{progress.readErrorCount}</strong> erros de leitura
        </span>
      </div>
    </section>
  );
}
