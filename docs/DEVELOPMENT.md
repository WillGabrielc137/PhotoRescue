# Desenvolvimento

Este documento resume o fluxo de desenvolvimento local do PhotoRescue.

## Stack

- Rust 2021 em workspace Cargo.
- Tauri 2 para desktop.
- React 19, TypeScript e Vite no frontend.
- Vitest para testes do frontend.
- SQLite via `rusqlite` para indice local de sessoes.
- Scripts PowerShell para desenvolvimento, build, empacotamento e atalho.

## Estrutura

```text
apps/desktop/              App desktop React/Tauri
apps/desktop/src/          Interface React
apps/desktop/src-tauri/    Backend Tauri e configuracao desktop
crates/                    Crates Rust internos
docs/                      Documentacao tecnica
scripts/                   Automacoes PowerShell
```

## Comandos principais

Instalar dependencias Node:

```powershell
npm install
```

Rodar em desenvolvimento:

```powershell
npm run dev
```

Rodar testes do frontend:

```powershell
npm run frontend:test
```

Rodar testes Rust:

```powershell
cargo test
```

Gerar executavel sem instalador:

```powershell
npm run build
```

Gerar executavel e instaladores:

```powershell
npm run package
```

Criar atalho local apos gerar/instalar o executavel:

```powershell
npm run create-shortcut
```

## Cuidados de desenvolvimento

- Nao versionar `node_modules/`, `target/`, `apps/desktop/dist/`, instaladores, logs, bancos SQLite locais ou sessoes de varredura.
- Nao usar uma unidade real com dados importantes para testes destrutivos ou experimentais.
- Manter a regra de seguranca: origem sempre em modo somente leitura e destino sempre em outra unidade.
- Testes ignorados no Rust podem exigir Windows, uma unidade real e execucao como administrador.
- Antes de publicar uma versao, rode pelo menos `npm run frontend:test`, `cargo test` e `npm run build`.

## Publicacao

O reposititorio remoto oficial e:

```text
https://github.com/WillGabrielc137/PhotoRescue.git
```

O branch principal esperado e `main`.
