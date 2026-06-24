interface Props {
  selectedCount: number;
  destination: string;
  busy: boolean;
  onChooseDestination: () => void;
  onRecover: () => void;
}

export function RecoveryBar({
  selectedCount,
  destination,
  busy,
  onChooseDestination,
  onRecover,
}: Props) {
  if (selectedCount === 0) return null;
  return (
    <section className="recovery-bar">
      <div>
        <strong>{selectedCount} arquivo(s) selecionado(s)</strong>
        <span>{destination || "Escolha o destino em outra unidade"}</span>
      </div>
      <button className="secondary" onClick={onChooseDestination} disabled={busy}>
        Alterar destino
      </button>
      <button
        className="primary"
        onClick={onRecover}
        disabled={busy || !destination}
      >
        Recuperar selecionados
      </button>
    </section>
  );
}

