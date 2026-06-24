# Arquitetura do PhotoRescue

## Visão geral

O PhotoRescue usa um processo desktop Tauri com módulos Rust independentes.
Isso mantém o handle privilegiado da unidade fora da interface web e evita
expor um servidor HTTP ou uma porta local.

```text
React/TypeScript
      │ comandos e eventos Tauri
      ▼
Orquestração desktop
  ├─ platform  ── abre \\.\X: somente para leitura
  ├─ scanner   ── detecta assinatura e estima limites
  ├─ index     ── grava SQLite na unidade segura
  ├─ logging   ── grava log na unidade segura
  └─ recovery  ── relê offsets e grava apenas no destino
```

## Fronteiras

### `photorescue-domain`

Modelos serializáveis compartilhados: volume, sessão, formato, candidato e
status. Não acessa disco nem banco de dados.

### `photorescue-platform`

Enumera letras de unidade, constrói o caminho bruto `\\.\X:`, abre esse caminho
com `OpenOptions::read(true).write(false)` e oferece o fluxo de elevação UAC.
Também consulta o tamanho de setor informado pelo Windows e expõe um leitor
lógico com cache. Esse leitor permite que scanner, prévia e recuperação usem
offsets arbitrários, enquanto as operações físicas no volume permanecem
alinhadas ao setor. Isso evita o erro Win32 87 em blocos sobrepostos.

### `photorescue-scanner`

Recebe qualquer fonte `Read + Seek`, portanto é testável com arquivos e buffers
sintéticos. O scanner:

1. lê blocos de 8 MiB com sobreposição;
2. detecta headers;
3. chama o carver do formato;
4. valida limites e partes da estrutura;
5. emite progresso e candidatos;
6. não possui nenhuma API de escrita.

### `photorescue-recovery`

Relê exatamente `offset..offset+tamanho`, grava um arquivo temporário criado com
`create_new`, sincroniza, renomeia e calcula SHA-256. Se ocorrer erro, remove
apenas o temporário criado no destino.

### `photorescue-index`

Mantém um SQLite por sessão com `journal_mode=WAL` e `synchronous=FULL`. A pasta
do banco é validada antes da abertura para nunca coincidir com a letra de
origem.

### `photorescue-logging`

Mantém um arquivo por sessão. Mensagens técnicas ficam no arquivo; a interface
recebe textos curtos por eventos.

## Modelo de ameaça e segurança

- A origem é tratada como não confiável: comprimentos e offsets são limitados.
- O scanner não recebe um handle gravável.
- Caminhos de volume enviados pela interface não são usados diretamente; o
  backend procura o identificador na enumeração feita no próprio Windows.
- O destino é revalidado antes da varredura e antes da recuperação.
- Arquivos finais não são abertos com truncamento.
- Falhas parciais deixam, no máximo, um temporário na unidade de destino.
- Pré-visualizações são lidas novamente da origem e retornadas em memória.

## Persistência

Tabela `scans`:

- origem, dispositivo, pasta segura;
- início, término e estado.

Tabela `candidates`:

- formato, offset e tamanho estimado;
- confiança e status;
- nome original opcional;
- caminho recuperado opcional.

## Decisões futuras

A etapa NTFS deve introduzir uma fonte de extents:

```text
MFT + bitmap de alocação
        │
        ├─ candidato contíguo ── recovery atual
        └─ candidato fragmentado ── novo leitor de extents
```

O contrato do serviço de recuperação deverá evoluir de um intervalo único para
uma lista ordenada de extents, preservando a regra de leitura exclusiva da
origem.
