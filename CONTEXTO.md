# Handoff — EcoRIP / PrintCraft

## Onde está

- Projeto: `C:\Users\erick\Documents\Craft Anything\app\app`
- App: **EcoRIP** (Rust + egui/eframe 0.29), UI em pt-BR, Windows.
- Repo: `https://github.com/frank1n4/PrintCraft` — público, branch `main`, commit `077b7af`.
- FlexiPRINT 19 (só referência): `C:\Program Files (x86)\SAi\FlexiPRINT 19 AM.CO.ZA Cloud Edition`
  - `Program\app.exe` = design; `Program\app2.exe` = Production Manager; driver `AmComSolventW2*`.
- VectorCraft: `C:\Program Files\ArtCraft\VectorCraft` (`vectorcraft-cli.exe`).

## Objetivo

Central de produção de uma impressora ecossolvente (XP600, mídia 1,80 m), com a arte feita no
VectorCraft: adicionar → prévia → ajustes → RIP → imprimir (ou pasta) → histórico.
O Flexi é referência de vocabulário; **não** é para clonar o motor dele.

## Estado atual

- Funciona: filas Espera/RIP/Impressão/Impressos, cores/colunas do Production Manager,
  aninhamento, painéis, prévia na mídia, estimativa de tinta/tempo/custo, RIP via CLI do
  VectorCraft, impressão GDI, persistência JSON e modo `--screenshot`.
- Fiel ao Production Manager na UI e nas regras; o motor é diferente (VectorCraft + GDI, não
  Jaws/RIP PostScript/driver SAi).
- Briefing completo de correções: `PROMPT-CORRECOES.md` (é a especificação oficial).

## P0 — corrigir primeiro

1. **Progresso falso**: `Job::progress()` usa tempo e trava em 95%; falta Cancelar.
2. **Tinta virtual**: é debitada ao imprimir (`actions.rs:378-389`); deve mudar só no −100 ml.
3. **DPI silencioso**: teto de 60 MP (`model.rs:18`) derruba 1,80×3,00 m de 720 para ~85 dpi;
   `RESOLUTIONS[..].1` usa só X (720×1440 vira 720×720).
4. **Tiling**: `layout()` só divide se `items.len() == 1`; com `copies > 1` estoura a mídia.
5. **Persistência**: `dirty = false` antes de gravar (`app.rs:148`); `Msg::Printers` zera dirty.
6. **Watcher**: só Hold/Print, mtime em segundos, ignora deleção e save durante RIP.
7. **Impressão**: carrega imagem inteira, sem `AbortDoc`, e a margem da prévia não bate com o RIP.
8. **Controles decorativos**: modo de cor, ponto, tinta branca, passes, velocidade e direção não
   mudam a saída — implementar, desabilitar ou rotular como estimativa.

## P1 — integração VectorCraft

- Usar `info` para `warnings`, `fonts` e `artboards` (hoje só a prancheta 0).
- Criar documento com `document.new` em mm, não `artboard.setProps`.
- Timeout/cancelamento no CLI; watcher robusto; re-vincular arquivo ausente.
- Alinhar formatos aceitos com o que o CLI lê.

## Decisões

- Não reimplementar Jaws/ICC/driver SAi; não clonar a UI MFC pixel a pixel.
- Testes de impressão só em pasta ou “Microsoft Print to PDF”; nunca na impressora física sem
  pedido explícito.
- Repo público, `main`; Git 2.56 e GitHub CLI 2.103 instalados, logado como `frank1n4`.

## Próximo passo

Executar o P0 de `PROMPT-CORRECOES.md`, começando por progresso/cancelamento, tinta virtual e
DPI/tiling; rodar `cargo build`, `cargo test` e o teste de integração com o VectorCraft.
