# Roadmap

O PhotoRescue ainda esta em desenvolvimento. Esta lista registra melhorias planejadas ou desejaveis sem afirmar que elas ja existem.

## Recuperacao e scanner

- Melhorar recuperacao em FAT32 e exFAT.
- Adicionar analise mais profunda de NTFS, MFT e mapa de alocacao.
- Melhorar suporte a arquivos fragmentados.
- Criar fluxo para gerar imagem de seguranca da unidade antes da varredura.
- Melhorar deteccao e recuperacao de formatos RAW de cameras.
- Ampliar validacoes estruturais de formatos suportados.

## Interface e experiencia do usuario

- Adicionar filtros manuais por tipo de arquivo.
- Melhorar pre-visualizacao de arquivos grandes.
- Criar relatorios mais completos por sessao.
- Melhorar mensagens de erro para casos de permissao, unidade indisponivel e falha fisica.
- Melhorar textos da interface e internacionalizacao.

## Qualidade e distribuicao

- Adicionar mais testes automatizados de scanner, recuperacao e interface.
- Criar dados sinteticos de teste para formatos suportados.
- Otimizar performance em unidades grandes.
- Melhorar instalador e assinatura de distribuicao.
- Documentar processo de release.

## Limitacoes conhecidas

- Recuperacao nao e garantida.
- Arquivos sobrescritos geralmente nao podem ser recuperados.
- Arquivos fragmentados podem falhar ou sair incompletos.
- Midias com defeito fisico podem falhar durante a leitura.
- Acesso bruto a volumes esta implementado apenas para Windows.
- O uso real pode exigir permissao de administrador.
