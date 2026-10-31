<div align="center">

# WPP Stats

**Estatísticas de uma conversa exportada do WhatsApp, calculadas no seu computador.**
Sem internet, sem login, sem banco de dados. O app mede a conversa, não lê.

Rust · Tauri 2 · Svelte · Windows 10/11

</div>

<p align="center">
  <img src="docs/img/visao-geral.png" alt="Visão geral do dashboard" width="100%">
</p>

### No app

|                                                                   |                                                         |
| ----------------------------------------------------------------- | ------------------------------------------------------- |
| <img src="docs/img/app-visao-geral.png" alt="Visão geral no app"> | <img src="docs/img/app-midias.png" alt="Mídias no app"> |
| <img src="docs/img/app-grupo-mensagens.png" alt="Grupo no app">   |                                                         |

## Download

**[Baixar o WPP Stats para Windows](../../releases/latest)** (arquivo `.exe`, não precisa instalar).

- Requer Windows 10 ou 11 com o WebView2, que já vem no Windows 11 e no Windows 10 atualizado.
- Como o executável não tem assinatura digital, o Windows pode mostrar "O Windows protegeu o computador". Clique em **Mais informações › Executar assim mesmo**.
- Confira o arquivo baixado com o SHA-256 publicado junto na página da versão.

## Privacidade em primeiro lugar

Uma conversa do WhatsApp é das coisas mais pessoais que alguém tem no computador. O app foi desenhado em torno disso:

- **Zero rede.** O app não faz nenhuma requisição. Não há cliente HTTP no binário e a política de segurança da janela (CSP) só permite falar com o próprio processo.
- **Mede, não lê.** O texto de cada mensagem existe na memória apenas durante o parsing, o tempo de contar bytes, caracteres, links e emojis. Depois é descartado. A interface nunca exibe uma mensagem. As únicas exceções, visíveis nos Destaques, são os emojis e as figurinhas mais usados.
- **Nada de texto em disco.** O cache guarda só números (horário, autor, tipo, tamanhos, durações). Relatórios e logs também.
- **Relatório anônimo.** O PDF pode ser exportado com "Pessoa A / Pessoa B" no lugar dos nomes e sem nomes de arquivo, pronto para compartilhar.
- **Prova de integridade.** Todo relatório traz o SHA-256 do arquivo da conversa, o fuso usado e o período, para que o resultado possa ser conferido depois.
- **Código aberto.** Tudo que o app faz com a sua conversa está neste repositório.

## O que ele mostra

|                                                                                                                  |                                                                                          |
| ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| <img src="docs/img/mensagens.png" alt="Aba Mensagens">                                                           | <img src="docs/img/horarios.png" alt="Aba Horários">                                     |
| **Mensagens:** tempo de resposta (mediana, 75% e 90%), caracteres, links, emojis, apagadas e mensagens por tipo. | **Horários:** mapa de calor dia da semana × hora e distribuição por dia e por hora.      |
| <img src="docs/img/audios.png" alt="Aba Áudios">                                                                 | <img src="docs/img/midias.png" alt="Aba Mídias">                                         |
| **Áudios:** quantidade, horas gravadas e duração média por pessoa, lidas do cabeçalho de cada arquivo.           | **Mídias:** contagem e tamanho por tipo, incluindo fotos e vídeos de visualização única. |

<p align="center">
  <img src="docs/img/destaques.png" alt="Aba Destaques" width="100%">
</p>

**Destaques:** dias de pico (Top 10), horário e dia preferidos, maior sequência de dias seguidos, maior intervalo sem conversa, emojis e figurinhas mais usados.

Tudo responde na hora a período (de/até e atalhos), fuso horário e filtro por pessoa. Exporta para **CSV** (pronto para o Excel), **JSON** e **PDF**, com opção de PDF anônimo.

### Grupos

Conversas em grupo funcionam do mesmo jeito: cada participante tem sua cor, os mais ativos aparecem nos cards e o restante é somado em "Demais".

|                                                                     |                                                                 |
| ------------------------------------------------------------------- | --------------------------------------------------------------- |
| <img src="docs/img/grupo-visao-geral.png" alt="Grupo: visão geral"> | <img src="docs/img/grupo-mensagens.png" alt="Grupo: mensagens"> |

### Tela inicial

<p align="center">
  <img src="docs/img/inicio.png" alt="Tela inicial" width="100%">
</p>

Arraste o `.zip` exportado pelo celular (ou a pasta já extraída) para qualquer lugar da janela. As conversas recentes ficam à direita, com favoritos.

### Relatório em PDF

Relatório de poucas páginas gerado no próprio app, sem navegador. No modo anônimo, os nomes viram "Pessoa A" e "Pessoa B" e os nomes de arquivo somem. A última página traz a integridade da análise e o SHA-256 da conversa.

|                                                     |                                                       |
| --------------------------------------------------- | ----------------------------------------------------- |
| <img src="docs/img/pdf-1.png" alt="PDF: resumo">    | <img src="docs/img/pdf-2.png" alt="PDF: por pessoa">  |
| <img src="docs/img/pdf-3.png" alt="PDF: destaques"> | <img src="docs/img/pdf-4.png" alt="PDF: integridade"> |

## Desempenho

O objetivo de projeto é que o gargalo seja o disco: nenhuma etapa pode ser mais lenta do que ler os bytes uma vez.

Medido com `wpp-stats --timings` (build release, AMD Ryzen 5 5600G, SSD, arquivos já no cache do sistema):

| Etapa                             | 1 milhão de mensagens<br><sub>81 MB de texto, 10 anos</sub> | Conversa típica<br><sub>23 mil mensagens, 3.900 mídias, 664 MB</sub> |
| --------------------------------- | ----------------------------------------------------------: | -------------------------------------------------------------------: |
| Listar a pasta                    |                                                      0,2 ms |                                                                 4 ms |
| Ler e fazer o parsing             |                                                     ~100 ms |                                                                 9 ms |
| Duração de ~2 mil áudios e vídeos |                                                           – |                                                                31 ms |
| Construir o índice                |                                                       12 ms |                                                               0,6 ms |
| Consultar um período              |                                                       10 ms |                                                               0,5 ms |

Trocar período, fuso ou pessoa não reprocessa nada: só reagrega o índice em memória.

## Como funciona

```mermaid
flowchart LR
    A[pasta ou .zip] --> B[listagem<br/>1 passada]
    B --> C[parsing do<br/>_chat.txt]
    C -. em paralelo .-> D[SHA-256]
    C --> E[hash join<br/>com as mídias]
    E --> F[duração por<br/>cabeçalho]
    F --> G[eventos]
    G --> H[cubo diário]
    H --> I[consultas]
    G --> J[(cache<br/>bincode + zstd)]
```

- **Parsing sem regex e sem `String`.** O arquivo é lido inteiro e tratado como `&[u8]`. Mensagens são separadas com `memchr`/`memmem` (SIMD) e o cabeçalho `[dd/mm/aaaa, HH:MM:SS]` é validado por posição fixa de byte. Os padrões de sistema, mídia e mensagem apagada (pt-BR e inglês, iPhone e Android) ficam numa tabela compilada uma vez em `aho-corasick`.
- **Formato de data deduzido.** `dd/mm` ou `mm/dd` é decidido pelos valores e, se ambíguo, pela interpretação que gera menos quebras de ordem cronológica.
- **Eventos em struct-of-arrays.** Cada mensagem vira poucas colunas numéricas (horário, autor, tipo, bytes, caracteres, tamanho, duração), sem alocação por mensagem.
- **Mídias por hash join.** A pasta é listada uma vez e cada `<anexado: ...>` é resolvido em O(1). Nomes são normalizados (NFC + minúsculas) e nomes corrompidos pela extração do Windows (CP850/CP437) são reparados. Faltantes e órfãos saem de graça.
- **Duração sem decodificar.** Ogg Opus: `pre_skip` do `OpusHead` e o `granule position` da última página. MP4/MOV: salto entre caixas até o `mvhd`. Um vídeo de 90 MB é resolvido lendo poucos KB.
- **Cubo diário.** Contadores `[dia][hora][pessoa][tipo]` montados numa passada linear. Um período vira um intervalo de índices por busca binária, e cada estatística é uma soma sobre os dias.
- **Sem ordenar o que não precisa.** Mediana e percentis do tempo de resposta com `select_nth_unstable` (O(n)). Top 10 por seleção parcial.
- **Fuso horário correto.** O horário do chat é convertido com a tabela de transições do `jiff` e busca binária, uma vez por troca de fuso. Horários ambíguos de horário de verão são contados no relatório.
- **Cache que se invalida sozinho.** Chave = caminho + SHA-256 da conversa + impressão digital da listagem. Ao reabrir, só a listagem é refeita.

## Estrutura

```
crates/core   parser, mídias, durações, índice, fuso, cache e exportação (CSV, JSON, PDF)
crates/cli    binário wpp-stats, camada fina sobre o core
app           Tauri 2 + Svelte 5 + ECharts (local, sem CDN)
```

Toda a lógica vive no `core`. O front-end só recebe agregados de poucos KB por consulta.

## Rodando

Requisitos: Windows 10/11, [Rust](https://rustup.rs) estável e Node.js 20+.

```bash
npm --prefix app install
npm --prefix app run tauri dev      # app em modo desenvolvimento
npm --prefix app run tauri build    # executável em target/release/wpp-stats-app.exe
```

Linha de comando:

```bash
cargo run --release -p wpp-stats -- "caminho/da/conversa" --timings
cargo run --release -p wpp-stats -- conversa.zip --from 2026-01-01 --to 2026-06-30 --json saida.json --csv pasta --pdf relatorio.pdf
```

Testes:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Como exportar a conversa

**iPhone:** abra a conversa › toque no nome do contato › Exportar conversa › Anexar mídia. Leve o `.zip` para o computador e solte na janela.

**Android:** abra a conversa › menu › Mais › Exportar conversa › Incluir arquivos de mídia.

O app aceita o `.zip` como veio ou a pasta já extraída, de conversas individuais ou em grupo.

## Licença

[MIT](LICENSE)
