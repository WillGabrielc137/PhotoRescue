interface Props {
  messages: string[];
}

export function ActivityLog({ messages }: Props) {
  return (
    <section className="panel log-panel">
      <div className="section-heading">
        <div>
          <span className="eyebrow">Atividade</span>
          <h2>O que está acontecendo</h2>
        </div>
      </div>
      <ol>
        {messages.slice(-8).map((message, index) => (
          <li key={`${message}-${index}`}>{message}</li>
        ))}
      </ol>
    </section>
  );
}

