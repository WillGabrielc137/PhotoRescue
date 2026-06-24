# PhotoRescue

## Sobre o projeto

PhotoRescue e um programa desktop em desenvolvimento para auxiliar na recuperacao de arquivos apagados, com foco inicial em fotos, imagens e midias removiveis como cartao SD, microSD e pendrives.

O objetivo do projeto e escanear uma unidade de origem em modo somente leitura, identificar possiveis imagens por assinaturas de arquivo e recuperar os candidatos selecionados para uma pasta segura em outra unidade.

Recuperacao de arquivos nunca e garantida. O resultado depende de fatores como:

- se os dados apagados ja foram sobrescritos;
- estado fisico do cartao, pendrive ou disco;
- sistema de arquivos usado pela midia;
- fragmentacao dos arquivos;
- tempo de uso da unidade apos a exclusao;
- erros de leitura ou setores defeituosos.

## Status do projeto

O PhotoRescue esta em andamento.

Estado atual:

- o programa ainda pode conter bugs;
- algumas recuperacoes podem falhar;
- algumas imagens podem ser recuperadas corrompidas, incompletas ou apenas parcialmente;
- novas melhorias ainda serao implementadas;
- a recuperacao deve ser feita com cuidado;
- os arquivos recuperados devem ser salvos em outra unidade, nunca na mesma unidade escaneada.

> **Aviso:** este software ainda esta em desenvolvimento. Ele pode apresentar limitacoes e comportamentos inesperados. Para aumentar as chances de recuperacao, pare de usar a midia afetada antes da varredura e salve os arquivos recuperados em uma unidade diferente.

## Funcionalidades atuais

Com base no codigo atual, o PhotoRescue possui:

- interface grafica desktop com React, TypeScript e Tauri;
- listagem de unidades disponiveis no Windows;
- identificacao de unidades removiveis;
- aviso visual sobre permissao limitada ou execucao como administrador;
- acao para reiniciar o programa com elevacao de administrador no Windows;
- escolha da unidade de origem;
- escolha de pasta segura de trabalho e recuperacao;
- validacao para impedir que a pasta de trabalho/destino fique na mesma unidade escaneada;
- abertura da origem em modo somente leitura;
- varredura normal;
- varredura profunda;
- cancelamento seguro da varredura;
- deteccao por assinatura/header de imagens JPEG, PNG, WebP, HEIC/AVIF, BMP e GIF;
- validacao parcial de estruturas internas de formatos como JPEG, PNG, GIF, BMP, WebP e HEIC;
- classificacao de candidatos como integros, parciais ou possivelmente corrompidos;
- exibicao de progresso, quantidade de candidatos, parciais, corrompidos e erros de leitura;
- tabela de candidatos encontrados;
- selecao manual dos candidatos que serao recuperados;
- selecao de todos os candidatos encontrados;
- pre-visualizacao em memoria de candidatos ate 20 MB;
- recuperacao apenas dos candidatos selecionados;
- organizacao dos arquivos recuperados por categoria;
- geracao de nomes seguros baseados no offset do arquivo;
- protecao contra sobrescrita de arquivos recuperados existentes;
- calculo de SHA-256 dos arquivos recuperados;
- indice SQLite por sessao;
- logs por sessao;
- scripts para desenvolvimento, build, empacotamento e criacao de atalho.

Nao ha, no codigo atual do caminho analisado, uma tela de filtro manual por tipo de arquivo. Os formatos suportados sao tratados internamente pelo scanner.

## Tecnologias utilizadas

Tecnologias identificadas no projeto:

- **Rust 2021** para o backend, scanner, recuperacao, indice, logs e integracao com o sistema;
- **Cargo workspace** para organizacao dos crates Rust;
- **Tauri 2** para aplicacao desktop;
- **React 19** para a interface grafica;
- **TypeScript** no frontend;
- **Vite 6** como servidor e build frontend;
- **Vitest** para testes do frontend;
- **SQLite** via `rusqlite` para indice das sessoes;
- **PowerShell** para scripts de desenvolvimento, build, empacotamento e atalho;
- **NSIS/WiX via Tauri bundle** para instaladores Windows, quando usado o build empacotado.

Bibliotecas Rust importantes:

- `rusqlite` para persistencia local;
- `serde` e `serde_json` para serializacao;
- `uuid` para identificadores de sessoes e candidatos;
- `chrono` para datas;
- `sha2` para hash SHA-256;
- `crc32fast` para validacao CRC de PNG;
- `fs2` para informacoes de espaco/volume;
- `thiserror` para erros tipados.

## Pre-requisitos

Para rodar o projeto em desenvolvimento, e necessario ter instalado:

- Windows, pois o acesso bruto a volumes esta implementado apenas para Windows;
- Node.js e npm;
- Rust e Cargo;
- toolchain nativa necessaria para compilar aplicacoes Tauri no Windows;
- permissao de administrador para ler unidades brutas como `\\.\X:`;
- WebView2 Runtime, quando necessario para aplicacoes Tauri no Windows.

Links uteis:

- Node.js: <https://nodejs.org/>
- Rust: <https://www.rust-lang.org/tools/install>
- Tauri - pre-requisitos: <https://tauri.app/start/prerequisites/>

## Instalacao

Clone o repositorio ou abra a pasta local do projeto:

```powershell
git clone https://github.com/WillGabrielc137/PhotoRescue.git
cd PhotoRescue
```

Instale as dependencias Node do workspace:

```powershell
npm install
```

O projeto tambem possui dependencias Rust gerenciadas pelo Cargo. Elas sao baixadas e compiladas automaticamente quando os comandos Cargo/Tauri forem executados.

Para compilar apenas os crates Rust:

```powershell
cargo build
```

## Como executar em modo desenvolvimento

O comando principal de desenvolvimento declarado no `package.json` da raiz e:

```powershell
npm run dev
```

Esse comando executa `scripts/dev.ps1`, que chama:

```powershell
npm --workspace @photorescue/desktop run tauri:dev
```

O app desktop usa Tauri. Durante o desenvolvimento, o frontend Vite roda na porta `1420`, conforme `apps/desktop/vite.config.ts` e `apps/desktop/src-tauri/tauri.conf.json`.

Tambem e possivel executar diretamente o script do workspace do app:

```powershell
npm --workspace @photorescue/desktop run tauri:dev
```

Para leitura bruta de unidades, execute o programa como administrador ou use a opcao da interface para reiniciar com elevacao.

## Como gerar o executavel ou instalador

### Gerar executavel de producao sem instalador

O projeto possui o script:

```powershell
npm run build
```

Esse comando executa `scripts/build.ps1`, que chama o build Tauri com `--no-bundle`:

```powershell
npm --workspace @photorescue/desktop run tauri:build -- --no-bundle
```

O executavel esperado e:

```text
target/release/PhotoRescue.exe
```

### Gerar executavel e instaladores Windows

O projeto tambem possui o script:

```powershell
npm run package
```

Esse comando executa `scripts/package.ps1`, que chama:

```powershell
npm --workspace @photorescue/desktop run tauri:build
```

Artefatos esperados pelo script:

```text
target/release/PhotoRescue.exe
target/release/bundle/msi/PhotoRescue_*.msi
target/release/bundle/nsis/PhotoRescue_*-setup.exe
```

A configuracao de empacotamento esta em:

```text
apps/desktop/src-tauri/tauri.conf.json
```

### Criar atalho na Area de Trabalho

Depois de gerar ou instalar o executavel, o projeto possui:

```powershell
npm run create-shortcut
```

Esse comando procura um executavel em locais conhecidos e cria um atalho `PhotoRescue.lnk` na Area de Trabalho do usuario.

## Como usar o programa

Fluxo recomendado para um usuario comum:

1. Pare de usar a midia afetada imediatamente.
2. Nao formate o cartao, pendrive ou disco.
3. Nao copie novos arquivos para a midia afetada.
4. Abra o PhotoRescue, preferencialmente como administrador.
5. Selecione a unidade ou cartao de origem.
6. Escolha o modo de varredura:
   - **Varredura normal:** prioriza candidatos completos e tende a ser mais rapida.
   - **Varredura profunda:** tenta encontrar imagens parciais e tolera mais erros, mas pode demorar mais.
7. Escolha uma pasta segura de trabalho e recuperacao em outra unidade.
8. Inicie a varredura.
9. Aguarde a conclusao ou cancele com seguranca, se necessario.
10. Confira a lista de candidatos encontrados.
11. Use a pre-visualizacao quando disponivel.
12. Selecione os arquivos que deseja recuperar.
13. Confirme ou altere a pasta de destino.
14. Clique em recuperar selecionados.
15. Confira os arquivos recuperados nas pastas de saida.
16. Consulte os logs da sessao se houver erro ou comportamento inesperado.

Alertas importantes:

- nunca salve arquivos recuperados no mesmo cartao/disco escaneado;
- evite usar a midia afetada antes da recuperacao;
- nao formate a midia antes da tentativa de recuperacao;
- nao copie novos arquivos para a midia afetada;
- se houver suspeita de defeito fisico, considere criar uma imagem da unidade antes de insistir em novas leituras.

## Funcionamento interno

O PhotoRescue usa uma aplicacao desktop Tauri. A interface React envia comandos para o backend Rust por IPC do Tauri. O backend enumera volumes, abre a unidade de origem em modo somente leitura, executa a varredura por assinaturas e salva metadados da sessao em uma pasta segura escolhida pelo usuario.

Fluxo simplificado:

```text
React/TypeScript
  -> comandos e eventos Tauri
  -> backend desktop Rust
  -> leitura somente leitura da unidade
  -> scanner por assinaturas
  -> indice SQLite e logs da sessao
  -> recuperacao dos candidatos selecionados
  -> escrita somente na pasta de destino
```

Principais areas internas:

- **Interface grafica:** renderiza selecao de unidade, modo de varredura, progresso, candidatos, pre-visualizacao, recuperacao e log de atividade.
- **Comandos Tauri:** conectam a interface aos servicos Rust.
- **Plataforma:** lista volumes Windows, identifica setor/unidade removivel, abre `\\.\X:` em modo somente leitura e solicita elevacao.
- **Scanner:** le blocos da unidade, procura assinaturas de imagens e emite candidatos.
- **Carvers:** validam ou estimam o tamanho dos arquivos a partir da estrutura de cada formato.
- **Indice:** grava sessoes e candidatos em SQLite.
- **Logs:** grava mensagens tecnicas por sessao.
- **Recuperacao:** relê os bytes da origem e grava os arquivos selecionados no destino seguro.

## Explicacao dos principais codigos

```text
package.json
```

Arquivo principal de scripts npm. Define comandos como `dev`, `build`, `package`, `create-shortcut` e `frontend:test`, alem do workspace `apps/desktop`.

```text
Cargo.toml
```

Define o workspace Rust e seus membros: aplicacao Tauri e crates internos de dominio, indice, logs, plataforma, recuperacao e scanner.

```text
scripts/common.ps1
```

Contem funcoes compartilhadas dos scripts PowerShell. Verifica dependencias como `node`, `npm`, `cargo` e `rustc`.

```text
scripts/dev.ps1
```

Executa o PhotoRescue em modo desenvolvimento chamando o script Tauri do workspace `@photorescue/desktop`.

```text
scripts/build.ps1
```

Gera o executavel de producao sem empacotar instaladores. Valida se `target/release/PhotoRescue.exe` foi criado.

```text
scripts/package.ps1
```

Gera o executavel e instaladores Windows via Tauri. Espera artefatos em `target/release`, `target/release/bundle/msi` e `target/release/bundle/nsis`.

```text
scripts/create-shortcut.ps1
```

Cria um atalho `PhotoRescue.lnk` na Area de Trabalho apontando para um executavel encontrado ou informado.

```text
apps/desktop/package.json
```

Define scripts e dependencias do app desktop: Vite, React, TypeScript, Vitest e Tauri CLI.

```text
apps/desktop/vite.config.ts
```

Configura o Vite com React, porta fixa `1420` e host `127.0.0.1`.

```text
apps/desktop/src/main.tsx
```

Ponto de entrada da interface React.

```text
apps/desktop/src/App.tsx
```

Componente principal da interface. Controla estado da aplicacao, lista volumes, inicia/cancela varreduras, recebe eventos, lista candidatos, solicita pre-visualizacao e dispara recuperacao.

```text
apps/desktop/src/api.ts
```

Camada de chamadas IPC para comandos Tauri como `list_volumes`, `start_scan`, `cancel_scan`, `preview_candidate` e `recover_candidates`.

```text
apps/desktop/src/types.ts
```

Define tipos TypeScript compartilhados pela interface, incluindo `VolumeInfo`, `RecoveryCandidate`, `ScanProgress`, `RecoveryProgress` e `ScanMode`.

```text
apps/desktop/src/components/ScanSetup.tsx
```

Tela de configuracao inicial. Permite escolher unidade, modo de varredura e pasta segura de trabalho/recuperacao.

```text
apps/desktop/src/components/SafetyNotice.tsx
```

Mostra aviso de seguranca e oferece acao para reiniciar como administrador quando a aplicacao nao esta elevada.

```text
apps/desktop/src/components/ScanStatus.tsx
```

Exibe progresso da varredura, estatisticas de candidatos e botao de cancelamento seguro.

```text
apps/desktop/src/components/CandidateTable.tsx
```

Mostra os candidatos encontrados, permite selecionar arquivos, selecionar todos e pedir pre-visualizacao.

```text
apps/desktop/src/components/RecoveryBar.tsx
```

Mostra a barra de recuperacao para os candidatos selecionados e permite alterar o destino.

```text
apps/desktop/src/components/ActivityLog.tsx
```

Mostra na interface as mensagens recentes de atividade recebidas durante o uso.

```text
apps/desktop/src/lib/format.ts
```

Funcoes auxiliares para formatar tamanhos em bytes e calcular percentual de progresso.

```text
apps/desktop/src-tauri/tauri.conf.json
```

Configuracao Tauri do produto, janela, seguranca, build frontend e empacotamento Windows.

```text
apps/desktop/src-tauri/src/main.rs
```

Ponto de entrada do binario Tauri. Chama a inicializacao definida em `lib.rs`.

```text
apps/desktop/src-tauri/src/lib.rs
```

Configura o Tauri, registra o plugin de dialogo, gerencia o estado global e expõe os comandos do backend para a interface.

```text
apps/desktop/src-tauri/src/commands.rs
```

Implementa os comandos Tauri principais: listagem de volumes, elevacao, inicio/cancelamento de varredura, listagem de candidatos, pre-visualizacao e recuperacao.

```text
apps/desktop/src-tauri/src/state.rs
```

Mantem as sessoes de varredura ativas em memoria, associadas por UUID.

```text
crates/photorescue-domain/src/lib.rs
```

Define os modelos compartilhados do dominio: formatos de imagem, status de candidatos, modo de varredura, candidatos, sessoes e informacoes de volume.

```text
crates/photorescue-platform/src/lib.rs
```

Implementa funcoes especificas de plataforma: listagem de volumes Windows, abertura de volume bruto em modo somente leitura, leitura alinhada por setor, verificacao de administrador e reinicio elevado.

```text
crates/photorescue-scanner/src/lib.rs
```

Implementa o scanner por assinaturas. Le a origem em blocos, detecta formatos suportados, gera eventos de progresso/candidato e respeita cancelamento.

```text
crates/photorescue-scanner/src/carvers.rs
```

Contem os carvers e validadores de formatos. Reconhece assinaturas e calcula/estima limites de JPEG, PNG, WebP, HEIC/AVIF, BMP e GIF.

```text
crates/photorescue-recovery/src/lib.rs
```

Implementa a recuperacao dos candidatos selecionados. Valida unidade de destino, cria pastas por categoria, grava temporario, sincroniza, renomeia, evita sobrescrita e calcula SHA-256.

```text
crates/photorescue-index/src/lib.rs
```

Gerencia o indice SQLite da sessao, com tabelas para `scans` e `candidates`.

```text
crates/photorescue-logging/src/lib.rs
```

Gerencia o arquivo de log da sessao, gravando mensagens com data, nivel e texto sanitizado.

## Estrutura de pastas

Estrutura simplificada do projeto analisado:

```text
PhotoRescue/
  apps/
    desktop/
      src/
        components/
        lib/
        App.tsx
        api.ts
        main.tsx
        types.ts
      src-tauri/
        capabilities/
        icons/
        src/
          commands.rs
          lib.rs
          main.rs
          state.rs
        tauri.conf.json
      package.json
      vite.config.ts
  crates/
    photorescue-domain/
    photorescue-index/
    photorescue-logging/
    photorescue-platform/
    photorescue-recovery/
    photorescue-scanner/
  docs/
    ARCHITECTURE.md
  scripts/
    build.ps1
    common.ps1
    create-shortcut.ps1
    dev.ps1
    package.ps1
  Cargo.toml
  Cargo.lock
  package.json
  package-lock.json
  LICENSE
  README.md
```

## Logs

Os logs existem no codigo atual.

Ao iniciar uma varredura, o backend cria uma sessao dentro da pasta segura escolhida pelo usuario:

```text
<pasta escolhida>/PhotoRescue/scan-<uuid>/
```

Dentro dessa sessao, os logs sao salvos em:

```text
<pasta escolhida>/PhotoRescue/scan-<uuid>/Logs/photorescue.log
```

Tambem e criado um indice SQLite em:

```text
<pasta escolhida>/PhotoRescue/scan-<uuid>/photorescue.sqlite
```

Os logs ajudam a entender:

- inicio e fim da varredura;
- configuracao da varredura;
- candidatos encontrados;
- erros de leitura;
- falhas ao indexar candidatos;
- arquivos recuperados;
- falhas de recuperacao.

A interface tambem mostra um log resumido de atividade recente, mas esse historico visual nao substitui o arquivo `photorescue.log`.

## Testes

O projeto possui testes de frontend com Vitest e testes Rust nos crates.

Para rodar os testes do frontend:

```powershell
npm run frontend:test
```

Esse script chama:

```powershell
npm --workspace @photorescue/desktop run test
```

Para rodar os testes Rust:

```powershell
cargo test
```

Alguns testes Rust marcados como `#[ignore]` exigem uma unidade Windows real e execucao como administrador. Esses testes nao rodam por padrao no `cargo test`.

## Cuidados e limitacoes

Limitacoes atuais importantes:

- a recuperacao nao e garantida;
- arquivos sobrescritos geralmente nao podem ser recuperados;
- arquivos fragmentados podem nao ser recuperados corretamente;
- imagens podem ser recuperadas parcialmente;
- imagens podem vir corrompidas;
- midias com defeito fisico podem falhar durante a leitura;
- o acesso bruto a volumes esta implementado apenas para Windows;
- a leitura de unidade bruta pode exigir administrador;
- a pre-visualizacao em memoria possui limite de 20 MB por candidato;
- nao ha filtro manual por tipo de arquivo na interface analisada;
- os nomes originais dos arquivos geralmente nao sao recuperados pelo scanner por assinatura;
- o scanner trabalha por assinatura/header e estimativa de limites, nao por restauracao completa do sistema de arquivos;
- a varredura profunda pode demorar bastante;
- o programa ainda esta em desenvolvimento.

## Roadmap / Melhorias futuras

Possiveis melhorias futuras:

- melhorar recuperacao em FAT32 e exFAT;
- melhorar suporte a estruturas NTFS, MFT e mapa de alocacao;
- melhorar recuperacao de arquivos fragmentados;
- criar imagem de seguranca da unidade antes da varredura;
- adicionar filtros manuais por tipo de arquivo;
- melhorar suporte a RAW de cameras;
- melhorar pre-visualizacao e tratamento de arquivos grandes;
- melhorar diagnostico de midias com erro fisico;
- otimizar desempenho em unidades grandes;
- adicionar mais testes automatizados;
- melhorar relatorios da sessao;
- melhorar instalador e processo de distribuicao;
- melhorar internacionalizacao e textos da interface;
- adicionar fluxo guiado para usuarios sem experiencia tecnica.

Uma versao mais organizada do roadmap esta em `docs/ROADMAP.md`.

## Documentacao complementar

- `docs/ARCHITECTURE.md`: visao tecnica da arquitetura interna.
- `docs/DEVELOPMENT.md`: comandos, fluxo de desenvolvimento e cuidados antes de contribuir.
- `docs/ROADMAP.md`: melhorias planejadas e limitacoes conhecidas.

## Aviso importante

> **Este software esta em desenvolvimento. Ele pode conter bugs, limitacoes e comportamentos inesperados. Use com cuidado. Para aumentar as chances de recuperacao, nao salve arquivos recuperados na mesma unidade escaneada e evite usar a midia afetada antes da recuperacao.**

## O que foi documentado

Este README documenta:

- objetivo e status do PhotoRescue;
- funcionalidades existentes no codigo atual;
- tecnologias e dependencias reais identificadas no projeto;
- pre-requisitos de ambiente;
- instalacao com `npm install` e build Rust com `cargo build`;
- execucao em desenvolvimento com `npm run dev`;
- build de executavel com `npm run build`;
- empacotamento com `npm run package`;
- criacao de atalho com `npm run create-shortcut`;
- fluxo de uso para usuario comum;
- funcionamento interno do frontend, Tauri e crates Rust;
- principais arquivos do projeto;
- estrutura de pastas;
- local dos logs e do indice SQLite;
- comandos de teste existentes;
- limitacoes atuais e melhorias futuras.

## Pontos nao confirmados automaticamente

O README foi baseado na estrutura e nos arquivos reais do projeto.

Pontos que dependem do ambiente local e nao podem ser garantidos apenas pela leitura do codigo:

- se todas as dependencias externas ja estao instaladas na maquina;
- se o build Tauri completo vai gerar MSI/NSIS sem erro no ambiente atual;
- se a leitura de uma unidade especifica funcionara sem erro fisico ou permissao insuficiente;
- se o WebView2 Runtime ja esta instalado no Windows;
- se todos os comandos vao concluir com sucesso em uma instalacao limpa sem antes instalar os pre-requisitos.
