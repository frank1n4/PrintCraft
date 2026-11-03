# Prompt — corrigir e endurecer o EcoRIP para produção real com VectorCraft

Você vai trabalhar no **EcoRIP**, um app Windows em Rust + egui (eframe 0.29), em
`C:\Users\erick\Documents\Craft Anything\app\app`.

Leia primeiro, nesta ordem:

1. `MECHANICS.md` e `mechanics.json` (o que foi extraído do FlexiPRINT e o que é estimado)
2. `src/model.rs`, `src/actions.rs`, `src/rip.rs`, `src/vc.rs`, `src/winprint.rs`
3. `src/ui.rs`, `src/ui_windows.rs`, `src/app.rs`, `src/main.rs`

Não reescreva o app do zero. A UI, as filas e a organização já estão boas; o trabalho é
corrigir o que engana o usuário e fechar a integração com o VectorCraft.

---

## 1. Contexto do produto

- O EcoRIP é a **central de produção** de uma impressora ecossolvente (XP600, mídia de 1,80 m).
- A arte é feita no **VectorCraft** (editor vetorial free, instalado em
  `C:\Program Files\ArtCraft\VectorCraft`, com `vectorcraft-cli.exe`).
- O objetivo é: adicionar arte → conferir prévia/dimensões → ajustar → RIP → imprimir (ou salvar
  os arquivos) → histórico.
- O FlexiPRINT 19 instalado é **apenas referência de vocabulário e de algumas regras**. O alvo
  **não** é clonar o Flexi pixel a pixel, nem reimplementar o RIP PostScript/Jaws dele.
- O usuário **nunca usou o FlexiPRINT**; não peça a ele para validar comportamento do Flexi.
  A especificação de “certo” é este documento.

Estado atual: compila, roda, tem as 4 filas, aninhamento, prévia na mídia, estimativa, RIP via
CLI do VectorCraft e impressão via GDI do Windows. O modo `--screenshot` é usado pelas checagens
do Craft Anything e deve continuar funcionando.

---

## 2. Regra de ouro: nada de controle decorativo

Hoje vários controles **não mudam a saída**. Isso é pior do que não existir, porque o usuário
acha que configurou a impressora.

Para cada ajuste, vale uma destas três opções:

1. o ajuste muda de verdade o arquivo gerado ou a impressão; ou
2. o ajuste fica desabilitado, com explicação; ou
3. o ajuste é rotulado **“informativo/estimativa”** e não entra em regras como se fosse real.

Vale para: modo de cor, tamanho do ponto, tinta branca, passes, velocidade, direção e nível de
tinta.

### O que cada controle faz hoje (fonte da verdade)

| Controle | Hoje | Deveria |
|---|---|---|
| Resolução | muda o raster, mas só usa o X e é cortada em silêncio pelo teto de 60 MP | respeitar X/Y ou remover modos não quadrados; mostrar DPI efetivo e avisar |
| Escala | muda o raster e o layout | ok |
| Cópias | vira N itens no layout | ok, mas corrigir o tiling (ver P0.4) |
| Girar / espelhar | aplicado no raster | ok |
| Mídia | entra na estimativa e na regra de aninhamento | ok; preço e nome são do usuário |
| Passes / velocidade / direção | só mudam a estimativa de tempo | rotular “estimativa” ou aplicar via DEVMODE (se possível) |
| Modo de cor / ponto / tinta branca | **não mudam nada** | remover da UI ou rotular claramente |
| Nível de tinta | só muda a estimativa de ml | rotular “estimativa” |
| Tinta virtual | é debitada ao imprimir | só o comando manual −100 ml / Encher |

---

## 3. O que já está bom — não regredir

- Layout de 4 filas (Espera, RIP, Impressão, Impressos), cores `#FFE0C1`, `#E1E1FF`, `#FFDFDF`,
  colunas Nome/Status/Configuração/Largura/Altura/Cópias, barra de progresso e prefixo `Nest `.
- Regras de aninhamento: mínimo 2, mesma mídia/modo/resolução/cor, limite de largura, não entrar
  em nest existente.
- Tinta virtual com −100 ml e confirmação.
- Listas do driver ecossolvente (velocidade, direção, passes, ponto, cor, tinta branca, níveis
  0–5) e limite 60% → 150%, inclinação 77.
- Prévia na mídia, estimativa de tinta/mídia/tempo/custo e painéis.
- Import/RIP/impressão em threads separadas; persistência em JSON.
- Modo `--screenshot` para as checagens.

---

## 4. P0 — Integridade (fazer primeiro)

### P0.1 Progresso e cancelamento honestos

- Hoje `Job::progress()` (`model.rs:224`) calcula por tempo decorrido e trava em 95%;
  `start_rip` chuta `expected_secs`. Isso não é progresso real.
- Trocar por estado **indeterminado** (“RIP…”, “Imprimindo página 1/2”) com tempo decorrido.
  Só mostrar `%` quando houver fonte real (páginas/bandas no GDI; progresso real do CLI, se
  existir).
- Adicionar **Cancelar** para importação, RIP e impressão:
  - encerrar o processo/thread;
  - voltar o job para Hold/Waiting;
  - apagar arquivos parciais;
  - não deixar job preso em “RIP 95%”.
- `Threads de RIP: 1` está fixo (`ui.rs:486`). Ou implementar um pool real, ou remover.

### P0.2 Separar tinta virtual de consumo

- Hoje `Msg::PrintDone` debita a estimativa da tinta virtual (`actions.rs:378-389`).
- Impressão **não** deve mexer na tinta virtual. Ela só muda pelo comando manual (−100 ml) e por
  “Encher”.
- Se quiser histórico, criar um “consumo estimado acumulado” separado.

### P0.3 Máquina de estados explícita

Definir e implementar:

```
Hold/Waiting → Rip/Ripping → Print/Ripped → Print/Printing → Done/Printed
                     ↓            ↓               ↓
                  Hold/Error   Hold/Error      Print/Error   (retry permitido)
```

- Enquanto `running`, só Cancelar é permitido.
- Editar ajuste que muda a saída (mídia, resolução, escala, cópias, rotação, espelho) invalida o
  RIP e volta para Hold/Waiting, apagando o RIP anterior.
- Editar ajuste que só muda estimativa (passes/velocidade/direção/nível de tinta) **não** deve
  forçar re-RIP.
- Excluir job remove prévia e arquivos de RIP do disco. Limpar órfãos em `%APPDATA%\EcoRIP` no
  startup.
- Ao abrir o app, jobs que estavam running voltam para Hold/Waiting (isso já existe) e arquivos
  parciais são limpos.

### P0.4 DPI efetivo visível e tiling correto

- `effective_dpi` (`rip.rs:14`) reduz a resolução em silêncio para caber em 60 MP
  (`model.rs:18`). Um 1,80 × 3,00 m a 720 dpi vira ~85 dpi sem o usuário saber.
- `RESOLUTIONS[..].1` usa só o X (`rip.rs:75`), então 720×1080 e 720×1440 saem como 720×720.
- Fazer:
  - mostrar “DPI efetivo” na propriedade e na prévia;
  - avisar quando for menor que o pedido;
  - respeitar X e Y, ou remover os modos não quadrados;
  - oferecer painelização em vez de baixar DPI, se possível.
- **Bug de tiling:** `layout()` (`model.rs:300-319`) só divide em painéis quando
  `items.len() == 1`. Com `copies > 1`, o job largo deixa de ser tiling e estoura a largura da
  mídia. Decidir e implementar: tiling vale por job (recomendado), independente de cópias.

### P0.5 Persistência atômica

- `save()` seta `dirty = false` **antes** de escrever (`app.rs:148`); se a escrita falhar, a
  alteração se perde.
- `Msg::Printers` seta `dirty = false` (`actions.rs:396-399`), podendo apagar uma alteração
  pendente.
- Fazer: gravar em `.tmp` + rename, manter um backup, colocar versão no JSON e, se corromper,
  carregar o backup e avisar. Não zerar `dirty` ao receber a lista de impressoras.

### P0.6 Arquivo alterado, ausente ou em uso

- `watch_files` (`actions.rs:70`) só olha Hold/Print, usa mtime em segundos, ignora deleção e
  ignora saves durante RIP/impressão.
- Fazer:
  - mtime em ms + tamanho (ou hash), debounce de ~300 ms;
  - observar todas as filas;
  - se mudou: invalidar prévia e RIP, marcar “desatualizado” e avisar;
  - se sumiu: “arquivo ausente”, bloquear RIP/impressão;
  - se estiver em uso: retry com backoff;
  - se o job estava em Print/Ripped e o arquivo mudou, voltar para Hold e exigir novo RIP.

### P0.7 Impressão

- `winprint.rs` carrega a imagem inteira na memória; banner grande pode estourar.
- Não há `AbortDoc` em falha e o erro do spooler não chega ao usuário.
- Fazer:
  - orçamento de memória com erro claro (não OOM);
  - `AbortDoc` em falha;
  - validar existência do arquivo antes de iniciar;
  - reportar o erro real do spooler;
  - definir margem: hoje a prévia desloca pelo `margin_left`, mas o RIP começa em 0
    (`rip.rs:105`). Escolher e documentar: margem é área não imprimível (saída começa em 0 e a
    prévia deve mostrar igual) **ou** offset real (canvas maior e conteúdo deslocado).

---

## 5. P1 — Integração com o VectorCraft (o coração do produto)

O contrato estável é o CLI. Use `vectorcraft-cli --help` e `vectorcraft-cli commands` como
referência; não invente flags.

1. **Descoberta e versão**
   - Procurar em `C:\Program Files\ArtCraft\VectorCraft`, `%ProgramFiles%\ArtCraft\VectorCraft`
     e `%LOCALAPPDATA%\Programs\ArtCraft\VectorCraft` (isso já existe).
   - Se possível, consultar o registro; checar versão mínima e dar erro claro se não achar.

2. **Timeout e cancelamento**
   - Todo `run` com timeout (import: ~2 min; RIP: proporcional ao tamanho, com teto).
   - Poder matar o processo e liberar a thread.
   - Nunca bloquear a UI.

3. **Warnings de importação e fontes**
   - `info` devolve `warnings`, `fonts`, `kinds` e `info.units`; hoje tudo é ignorado.
   - Mostrar no job: “importado com avisos”, “fonte ausente”, “PDF lido pela prévia”.
   - Por padrão, **bloquear o RIP** quando o import foi lossy, com opção “RIP mesmo assim”.
   - Exibir as fontes usadas e alertar se não estiverem instaladas.

4. **Múltiplas pranchetas**
   - `info.artboards[]` pode ter várias; hoje só a 0 é usada (`vc.rs:60`).
   - Adicionar seletor “Prancheta” no job e usar `convert --artboard I`.
   - Alternativa: um job por prancheta, com nome “arquivo — prancheta N”.

5. **Novo desenho**
   - Hoje `vc::new_document` usa `artboard.setProps` (`vc.rs:76`), que deixa o documento com o
     tamanho padrão e só muda a prancheta.
   - Usar `run --commands` com `document.new` (`width`/`height` em mm, `units: "Millimeters"`,
     `artboards: 1`), exportar e abrir no editor.
   - Definir título/nome do documento.

6. **Unidades e transformações**
   - `info` devolve o `rect` em pontos do documento (confirmado com um doc de 1000 mm =
     2834.6 pt). Manter `pt ↔ mm`, mas validar com um documento criado em milímetros.
   - `convert --scale` é **pixels por ponto**; manter `dpi / 72` e testar 720/1440.

7. **Watcher / evento ao vivo**
   - Preferência: watcher de arquivo robusto (mtime ms + tamanho/hash, debounce).
   - Se precisar de evento ao vivo, avaliar o MCP do VectorCraft
     (`vectorcraft-cli mcp`, app rodando com `--control 7979`), mas **não** bloquear a entrega
     nisso.

8. **Abrir / editar / re-vincular**
   - “Abrir no VectorCraft” deve funcionar para o job selecionado.
   - Se o path estiver vazio ou o arquivo sumiu, oferecer “Localizar arquivo…” e re-vincular,
     em vez de abrir o editor em branco.

9. **Prévia e invalidação**
   - Atualizar a prévia quando o arquivo for salvo.
   - Se o job estava em Print/Ripped, voltar para Hold e exigir novo RIP.
   - Mostrar “prévia desatualizada” enquanto a atualização não termina.

10. **Formatos**
    - Alinhar `VECTOR_EXTS` com o que o CLI lê (inclui `drawcraft`, `vctemplate`, `psd`, `dxf`,
      `ai`, `eps`, `ps`, `tif` etc.).
    - Validar por `info` em vez de lista fixa; se o CLI recusar, mostrar o erro dele.

11. **CLI**
    - Usar `info` para dimensões/warnings/fontes/pranchetas.
    - Usar `convert` ou `run --export` para o raster; `--scale` em pixels por ponto.
    - Passar caminhos como argumentos (nunca montar shell string).

---

## 6. P2 — Produção (o que falta para operar de verdade)

- **Aninhamento:** rotação 90° opcional (default desligado), gap configurável (hoje 10 mm fixo)
  e, se o usuário marcar, respeitar a direção da mídia/grão.
- **Painéis:** sobreposição configurável (hoje 10 mm fixo), marcas de emenda/ordem e
  painelização também por comprimento quando o driver limitar o tamanho da página.
- **Fila:** prioridade, pausar/retomar, reimprimir, duplicar.
- **Setups/impressoras:** decidir se haverá mais de um setup; se não, deixar explícito “um
  setup” e separar “modelo da impressora” (texto) de “impressora do Windows” (saída).
- **Estimativa:** rotular como estimativa, mostrar premissas (cobertura da prévia, 3 pL, E do
  nível), permitir corrigir preço/ml e configurar a moeda (hoje “R$” fixo).
- **Log:** arquivo rotativo de jobs/erros, “Abrir pasta de saída” e histórico de impressos com
  data/hora e custo.

---

## 7. P3 — UX

- Avisos inline para ajustes não aplicados (“este modo de cor não altera a saída”).
- Colunas ordenáveis, filtro e atalhos.
- Consistência em pt-BR: “Setup” vs “Configuração”; “Nest” vs “Aninhado”.
- Estados vazios e dicas.
- Acessibilidade básica: contraste, foco, navegação por Tab.

---

## 8. Fora de escopo (não tentar)

- Reescrever o RIP PostScript/Jaws ou o motor do Flexi.
- Implementar ICC/halftone/separacão de canais própria.
- Falar direto com a cabeça de impressão pelo driver protegido/criptografado.
- Contornar licenciamento da SAi.
- Clonar a UI MFC pixel a pixel.
- Alterar a instalação do VectorCraft ou os arquivos do usuário.

---

## 9. Critérios de aceite (rodar e anexar evidência)

1. `cargo build` e `cargo test` passam; o teste de integração com o VectorCraft continua
   passando.
2. Criar 600×400 mm no app → o VectorCraft abre com página 600×400 (não Letter/A4).
3. Adicionar um `.vectorcraft` com 2 pranchetas → escolher a prancheta; prévia e dimensões
   corretas.
4. Salvar o arquivo no VectorCraft com o job em Hold, em Rip e em Print → o app detecta,
   invalida e avisa nos 3 casos.
5. Apagar o arquivo → job “arquivo ausente”, RIP/impressão bloqueados.
6. Cancelar RIP e impressão → volta para Hold, sem arquivo parcial e sem thread pendurada.
7. 1,80 × 3,00 m a 720 dpi → mostra DPI efetivo e/ou painéis; não estoura memória; impressão não
   trava.
8. Editar resolução/escala/cópias → RIP invalidado; editar passes → não invalida.
9. Fechar o app no meio do RIP → reabrir: job em Hold, sem “RIP 95%” fantasma.
10. Imprimir para pasta → PNG com dimensões físicas e ppm corretos; imprimir na impressora →
    tamanho real confere.
11. Excluir job → prévia/RIP removidos; nenhum órfão em `%APPDATA%\EcoRIP`.
12. `--screenshot` continua gerando a imagem de checagem.

---

## 10. Como trabalhar

- Passos pequenos; build + test depois de cada mudança.
- Não reescrever a UI inteira; preservar o que está na seção 3.
- Se mudar um valor de `MECHANICS.md`/`mechanics.json`, atualizar os dois e explicar.
- Evitar dependências novas; se adicionar (ex.: `notify`), justificar.
- Windows 10/11 x64, sem exigir admin.
- **Nunca** disparar job na impressora física durante os testes; usar saída para pasta ou
  “Microsoft Print to PDF”. Só imprimir na ecossolvente se o usuário pedir.
- Deixar um `DECISOES.md` curto com as escolhas (margem, rotação, DPI, tiling, rótulos de
  estimativa).
- No final, listar o que ficou fora e por quê.

## 11. Entrega

1. Código corrigido, compilando.
2. Testes novos para: máquina de estados, invalidação por mudança de arquivo, tiling com
   cópias, limite de tinta e parsing do `info`.
3. `DECISOES.md`.
4. Um resumo de 10 linhas: o que mudou, o que foi testado e o que ainda é limitação.
