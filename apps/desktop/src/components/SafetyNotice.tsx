interface Props {
  elevated: boolean;
  onElevate: () => void;
}

export function SafetyNotice({ elevated, onElevate }: Props) {
  return (
    <section className="safety-card" aria-label="Aviso de segurança">
      <div className="safety-icon" aria-hidden="true">
        !
      </div>
      <div>
        <h2>Proteja os dados antes de escanear</h2>
        <p>
          Pare de usar a unidade afetada. Escolha uma pasta de trabalho em outra
          unidade física, de preferência um SSD externo ou pendrive. O
          PhotoRescue abre a origem somente para leitura.
        </p>
      </div>
      {!elevated && (
        <button className="secondary danger" onClick={onElevate}>
          Reiniciar como administrador
        </button>
      )}
    </section>
  );
}

