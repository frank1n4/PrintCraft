# Mechanics sheet — EcoRIP (rebuild of the FlexiPRINT 19 production side)

Evidence comes from the installed FlexiPRINT 19 AM.CO.ZA Cloud Edition. `app.exe` (the design
program the project points at) is a thin MFC shell (294 KB) whose printing lives in other DLLs; the
part that *operates the printer* is `app2.exe` ("Production Application", the Production Manager),
plus the eco-solvent output driver `OutputDrivers\AmComSolventW2*.dll` and the XML files beside it.
Nothing was run; everything below is read from files or decompiled with Ghidra.

## Queues (app2.exe)

| Rule | Value | Source |
|---|---|---|
| Hold queue id | 1 | `FUN_0046cd50` (APPSHoldJobListView ctor, `param_1[0x2b] = 1`) |
| RIP queue id | 2 | `FUN_0046cfd0` (APPSRIPJobListView ctor) |
| Print queue id | 3 | `FUN_0046ceb0` (APPSPrintJobListView ctor) |
| Hold queue background | COLORREF 0xC1E0FF = rgb(255,224,193) | `FUN_0046cd50`, key `HOLD_QUEUE_COLOR` |
| RIP queue background | COLORREF 0xFFE1E1 = rgb(225,225,255) | `FUN_0046cfd0`, key `RIP_QUEUE_COLOR` |
| Print queue background | COLORREF 0xDFDFFF = rgb(255,223,223) | `FUN_0046ceb0`, key `PRINT_QUEUE_COLOR` |
| Status column progress bar | fill COLORREF 0xFF8080 = rgb(128,128,255) on white, 1 px inset, sunken edge | `FUN_00450dc0` |
| Progress parsing | the number before `%` in the status text, clamped to 100 | `FUN_00450dc0` (`atoi(p-3)`, `if (100 < n) n = 100`) |
| Status text left padding | 6 px | `FUN_00450dc0` (`local_390.left = left + 6`) |
| Nested job name prefix | `"Nest "` | `FUN_00450dc0`, string 0x49af10 |
| Selected row | system highlight colour, white text | `FUN_00450dc0` (`GetSysColor(0xd)`, text 0xffffff) |
| Queue columns | Name, Status, Setup, Width, Height, Copies | app2.exe .rsrc 0x503834–0x50384e, 0x506048 |
| Virtual ink command | decrements by 100 ml after a Yes/No confirmation | `FUN_0042f820` (command 0x7d2) |
| Status bar | `Ready`, `RIP threads: %d` | .rsrc 0x5066f8, 0x5068e2 |

### Nesting rules (app2.exe string table 0x506c54, 0x507b36)
- At least 2 jobs must be selected.
- All jobs must share media, print mode, resolution and colour setup.
- A job wider than the media can't be nested (tiling is per job only).
- Jobs can't be added to an existing nest: unnest first.
- Jobs with cut registration marks can't be nested.
- The original places items with `NestAlgo.dll!FAutoNest` (0x1000b850): a shape-packing
  routine run with a tolerance ≈ 0.005 (double 0x3F747064ECE9A2C6) and 100 iterations, which
  also returns a rotation for each item. The gap comes from the caller and was not found.
  The rebuild uses a simpler row ("shelf") layout without automatic rotation, with a
  **10 mm gap — estimated**.

## Eco-solvent driver options (AmComSolventW2LOC.dll, Portuguese UI)
- Velocidade: Alto, Meio, Baixo
- Direção: Unidirecional, Bidirecional
- Passo (passes): 1,2,3,4,5,6,7,8,9,10,12,13,14,15,16,20,24,32
- Tamanho do ponto: Ponto grande, Ponto médio, Ponto pequeno, Modo misto
- Modo de cor: CMYK, CMY, Tons de Cinza, CMYKLcLm, CMYKLcLmLkLLk, … (2 bits / 3 níveis variants)
- Tinta branca / verniz: Nenhum, Substrato, Cores Spot, Cor de baixo, Cor de preenchimento
- Resolutions are inside the encrypted `AmComSolventW2Des.enx`: the rebuild offers
  720×720, 720×1080, 720×1440, 1440×1440 — **estimated** (common XP600 head modes).

## Ink levels (OutputDrivers\InkMixing.xml, InkType "Eco-Solvent")
| Name | MaxInkVolumeStart | MaxInkVolumeEnd | MaxSlope | MixType |
|---|---|---|---|---|
| 0 – EcoSave (Strong) | 60 | 150 | 77 | L=0 H=16 S=25 E=45 |
| 1 – EcoSave | 60 | 150 | 77 | L=0 H=16 S=25 E=85 |
| 2 – Medium | 60 | 150 | 77 | L=0 H=20 S=30 E=95 |
| 3 – Balanced (default) | 60 | 150 | 77 | L=0 H=25 S=35 E=100 |
| 4 – High | 60 | 150 | 77 | L=7 H=100 S=40 E=100 |
| 5 – Maximum | 60 | 150 | 77 | L=15 H=100 S=45 E=100 |

The rebuild applies the limit to the total of the four channels: unchanged up to 60 %, then
growing at 77 % of the extra coverage, capped at 150 % (how the original bends the curve between
start and end is not in the XML — the bend shape is **estimated**, the three numbers are not).

## Ink estimate
- Drop volume: 3.0 (small = medium = large), Default vendor/model in `InkVolumeDefaults.xml`;
  model-specific values are encrypted. Read as picolitres.
- Rebuild formula (own, **estimated**): ml per channel = area_m² × (dpiX/0.0254) × (dpiY/0.0254)
  × coverage × 3.0 pL × 1e-9 × (E/100 of the chosen ink level) × copies.
- Coverage per channel comes from the job's preview (RGB → CMYK, averaged over the artboard).

## Not from the original (rebuild choices, all estimated)
- Print speed: 32 / passes m²/h × speed (Alto 1.0, Meio 0.8, Baixo 0.6) × direction
  (Bidirecional 1.0, Unidirecional 0.55).
- Tiling overlap: 10 mm.
- RIP raster limit: 60 megapixels per output (the effective dpi drops for big jobs).
- Virtual ink bottle: 1000 ml per colour when refilled.
- Media list and prices per m², ink price per ml.

## VectorCraft link (C:\Program Files\ArtCraft\VectorCraft)
- `vectorcraft-cli info FILE` → artboard rect in points (1 pt = 25.4/72 mm).
- `vectorcraft-cli convert IN OUT.png --scale N` → raster at N pixels per point (dpi = 72·N).
- `vectorcraft-cli run --commands - --export NEW.vectorcraft` with `artboard.setProps`
  `{index:0,x:0,y:0,width,height}` (points) creates a blank document at the media size.
- `vectorcraft.exe FILE` opens it for editing; the rebuild watches the file and refreshes the job
  when it is saved.
