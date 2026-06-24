import { formatBytes } from "../lib/format";
import type { RecoveryCandidate } from "../types";

interface Props {
  candidates: RecoveryCandidate[];
  selected: Set<string>;
  previewUrl: string | null;
  previewCandidateId: string | null;
  onToggle: (id: string) => void;
  onToggleAll: () => void;
  onPreview: (candidate: RecoveryCandidate) => void;
}

const statusLabel = {
  found: "Integro",
  partial: "Parcial",
  recovered: "Recuperado",
  failed: "Falhou",
  corrupted: "Possível corrupção",
};

export function CandidateTable({
  candidates,
  selected,
  previewUrl,
  previewCandidateId,
  onToggle,
  onToggleAll,
  onPreview,
}: Props) {
  if (candidates.length === 0) return null;

  return (
    <section className="results-layout">
      <div className="panel results-panel">
        <div className="section-heading">
          <div>
            <span className="eyebrow">Etapa 3</span>
            <h2>Arquivos candidatos</h2>
          </div>
          <button className="text-button" onClick={onToggleAll}>
            {selected.size === candidates.length
              ? "Limpar seleção"
              : "Selecionar todos"}
          </button>
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th aria-label="Selecionar" />
                <th>Tipo</th>
                <th>Tamanho</th>
                <th>Posição no disco</th>
                <th>Confiança</th>
                <th>Status</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {candidates.map((candidate) => (
                <tr
                  key={candidate.id}
                  className={selected.has(candidate.id) ? "selected-row" : ""}
                >
                  <td>
                    <input
                      type="checkbox"
                      checked={selected.has(candidate.id)}
                      onChange={() => onToggle(candidate.id)}
                      aria-label={`Selecionar ${candidate.id}`}
                    />
                  </td>
                  <td>
                    <span className={`format-tag ${candidate.format}`}>
                      {candidate.format.toUpperCase()}
                    </span>
                  </td>
                  <td>{formatBytes(candidate.estimatedSize)}</td>
                  <td className="mono">
                    0x{candidate.offset.toString(16).toUpperCase()}
                  </td>
                  <td>{candidate.confidence}%</td>
                  <td>
                    <span className={`status ${candidate.status}`}>
                      {statusLabel[candidate.status]}
                    </span>
                  </td>
                  <td>
                    <button
                      className="preview-button"
                      onClick={() => onPreview(candidate)}
                    >
                      Visualizar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <aside className="panel preview-panel">
        <span className="eyebrow">Pré-visualização</span>
        {previewUrl ? (
          <>
            <div className="preview-frame">
              <img src={previewUrl} alt="Prévia do arquivo candidato" />
            </div>
            <small>
              Candidato {previewCandidateId?.slice(0, 8)}. A prévia não garante
              que o arquivo esteja íntegro.
            </small>
          </>
        ) : (
          <div className="empty-preview">
            <span aria-hidden="true">▧</span>
            <p>Selecione “Visualizar” para tentar abrir a imagem em memória.</p>
          </div>
        )}
      </aside>
    </section>
  );
}
