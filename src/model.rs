//! Data, presets and the rules from MECHANICS.md.
use serde::{Deserialize, Serialize};

// ---- values from the mechanics sheet -------------------------------------------------------
pub const HOLD_QUEUE_COLOR: [u8; 3] = [255, 224, 193]; // 0x46cd50
pub const RIP_QUEUE_COLOR: [u8; 3] = [225, 225, 255]; // 0x46cfd0
pub const PRINT_QUEUE_COLOR: [u8; 3] = [255, 223, 223]; // 0x46ceb0
pub const PROGRESS_COLOR: [u8; 3] = [128, 128, 255]; // 0x450dc0
pub const PROGRESS_MAX: f32 = 100.0; // 0x450dc0
pub const STATUS_TEXT_PADDING: f32 = 6.0; // 0x450dc0
pub const NEST_PREFIX: &str = "Nest "; // 0x450dc0
pub const VIRTUAL_INK_DECREMENT_ML: f32 = 100.0; // 0x42f820
pub const MIN_JOBS_TO_NEST: usize = 2; // string table 0x506c54
pub const NEST_GAP_MM: f32 = 10.0; // estimated
pub const TILE_OVERLAP_MM: f32 = 10.0; // estimated
pub const DROP_VOLUME_PL: f32 = 3.0; // InkVolumeDefaults.xml
pub const PRINT_SPEED_BASE: f32 = 32.0; // m2/h at 1 pass, estimated
pub const RIP_RASTER_LIMIT_MP: f32 = 60.0; // estimated
pub const INK_BOTTLE_ML: f32 = 1000.0; // estimated
pub const POINTS_PER_INCH: f32 = 72.0; // VectorCraft
pub const MM_PER_INCH: f32 = 25.4;

pub fn pt_to_mm(pt: f32) -> f32 {
    pt * MM_PER_INCH / POINTS_PER_INCH
}
pub fn mm_to_pt(mm: f32) -> f32 {
    mm * POINTS_PER_INCH / MM_PER_INCH
}

// ---- driver option lists (AmComSolventW2LOC.dll) -------------------------------------------
pub const SPEEDS: [&str; 3] = ["Alto", "Meio", "Baixo"];
pub const SPEED_FACTOR: [f32; 3] = [1.0, 0.8, 0.6]; // estimated
pub const DIRECTIONS: [&str; 2] = ["Unidirecional", "Bidirecional"];
pub const DIRECTION_FACTOR: [f32; 2] = [0.55, 1.0]; // estimated
pub const PASSES: [u32; 18] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 20, 24, 32];
pub const DOT_SIZES: [&str; 4] = ["Ponto grande", "Ponto médio", "Ponto pequeno", "Modo misto"];
pub const COLOR_MODES: [&str; 6] = [
    "CMYK",
    "CMYK (2 bits)",
    "CMYKLcLm",
    "CMYKLcLm (2 bits)",
    "CMY",
    "Tons de Cinza",
];
pub const WHITE_MODES: [&str; 5] = [
    "Nenhum",
    "Substrato",
    "Cores Spot",
    "Cor de baixo",
    "Cor de preenchimento",
];
/// (label, dpi x, dpi y) — estimated, the real list is encrypted in the driver.
pub const RESOLUTIONS: [(&str, u32, u32); 4] = [
    ("720 x 720", 720, 720),
    ("720 x 1080", 720, 1080),
    ("720 x 1440", 720, 1440),
    ("1440 x 1440", 1440, 1440),
];
/// Eco-Solvent ink levels from InkMixing.xml: (name, E value of MixType).
pub const INK_LEVELS: [(&str, f32); 6] = [
    ("0 – EcoSave (forte)", 45.0),
    ("1 – EcoSave", 85.0),
    ("2 – Médio", 95.0),
    ("3 – Equilibrado (padrão)", 100.0),
    ("4 – Alto", 100.0),
    ("5 – Máximo", 100.0),
];
pub const DEFAULT_INK_LEVEL: usize = 3;
/// Ink limit of every Eco-Solvent level in InkMixing.xml (percent of one full channel).
pub const MAX_INK_VOLUME_START: f32 = 60.0;
pub const MAX_INK_VOLUME_END: f32 = 150.0;
pub const MAX_SLOPE: f32 = 77.0;

/// Total ink (sum of the channels, 0..4) after the ink limit: below the start nothing changes,
/// above it the curve bends with the slope (in percent) until it reaches the end limit.
pub fn limit_total_ink(total: f32) -> f32 {
    let t = total * 100.0;
    let limited = if t <= MAX_INK_VOLUME_START {
        t
    } else {
        (MAX_INK_VOLUME_START + (t - MAX_INK_VOLUME_START) * MAX_SLOPE / 100.0).min(MAX_INK_VOLUME_END)
    };
    limited / 100.0
}
pub const ROTATIONS: [u32; 4] = [0, 90, 180, 270];
pub const INK_NAMES: [&str; 4] = ["Ciano", "Magenta", "Amarelo", "Preto"];
pub const INK_COLORS: [[u8; 3]; 4] = [[0, 160, 227], [228, 0, 124], [255, 221, 0], [35, 31, 32]];

#[derive(Clone, Serialize, Deserialize)]
pub struct Media {
    pub name: String,
    pub price_m2: f32,
}

pub fn default_media() -> Vec<Media> {
    // estimated sample list, editable in the setup
    vec![
        Media { name: "Vinil adesivo brilho".into(), price_m2: 9.0 },
        Media { name: "Vinil adesivo fosco".into(), price_m2: 9.5 },
        Media { name: "Lona frontlight 440 g".into(), price_m2: 7.0 },
        Media { name: "Lona backlight".into(), price_m2: 14.0 },
        Media { name: "Vinil perfurado".into(), price_m2: 16.0 },
        Media { name: "Papel fotográfico".into(), price_m2: 11.0 },
    ]
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Queue {
    Hold,
    Rip,
    Print,
    Done,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Status {
    Importing,
    Waiting,
    Ripping,
    Ripped,
    Printing,
    Printed,
    Error(String),
}

impl Status {
    pub fn label(&self) -> String {
        match self {
            Status::Importing => "Lendo arquivo...".into(),
            Status::Waiting => "Em espera".into(),
            Status::Ripping => "Processando...".into(),
            Status::Ripped => "Pronto para imprimir".into(),
            Status::Printing => "Imprimindo...".into(),
            Status::Printed => "Impresso".into(),
            Status::Error(e) => format!("Erro: {e}"),
        }
    }
    pub fn running(&self) -> bool {
        matches!(self, Status::Ripping | Status::Printing | Status::Importing)
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSettings {
    pub media: usize,
    pub resolution: usize,
    pub color_mode: usize,
    pub passes: usize, // index into PASSES
    pub speed: usize,
    pub direction: usize,
    pub dot: usize,
    pub ink_level: usize,
    pub white: usize,
    pub scale_pct: f32,
    pub copies: u32,
    pub mirror: bool,
    pub rotate: usize, // index into ROTATIONS
}

impl Default for JobSettings {
    fn default() -> Self {
        Self {
            media: 0,
            resolution: 2,
            color_mode: 0,
            passes: 3, // 4 passes
            speed: 1,
            direction: 1,
            dot: 3,
            ink_level: DEFAULT_INK_LEVEL,
            white: 0,
            scale_pct: 100.0,
            copies: 1,
            mirror: false,
            rotate: 0,
        }
    }
}

impl JobSettings {
    /// Nesting rule: same media, print mode, resolution and colour setup.
    pub fn nest_key(&self) -> (usize, usize, usize, usize, usize, usize) {
        (self.media, self.resolution, self.color_mode, self.passes, self.speed, self.direction)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub art_w_mm: f32,
    pub art_h_mm: f32,
    pub queue: Queue,
    pub status: Status,
    pub settings: JobSettings,
    pub coverage: [f32; 4],
    pub preview: Option<String>,
    pub ripped: Vec<String>,
    pub rip_dpi: f32,
    pub mtime: u64,
    pub nest: Option<u64>,
    #[serde(skip)]
    pub started: Option<std::time::Instant>,
    #[serde(skip)]
    pub expected_secs: f32,
    #[serde(default)]
    pub demo_progress: Option<f32>,
}

impl Job {
    /// Output size after scale and rotation.
    pub fn out_size_mm(&self) -> (f32, f32) {
        let s = self.settings.scale_pct / 100.0;
        let (w, h) = (self.art_w_mm * s, self.art_h_mm * s);
        if ROTATIONS[self.settings.rotate] % 180 == 90 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// Progress 0..=100 shown in the status column.
    pub fn progress(&self) -> Option<f32> {
        if let Some(p) = self.demo_progress {
            return Some(p.min(PROGRESS_MAX));
        }
        if !self.status.running() {
            return None;
        }
        let started = self.started?;
        let t = started.elapsed().as_secs_f32();
        let exp = self.expected_secs.max(0.5);
        Some((t / exp * 100.0).min(95.0).min(PROGRESS_MAX))
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Output {
    WindowsPrinter(String),
    Folder(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Setup {
    pub name: String,
    pub printer_model: String,
    pub media_width_mm: f32,
    pub margin_left_mm: f32,
    pub margin_right_mm: f32,
    pub output: Output,
    pub print_while_rip: bool,
    pub ink_ml: [f32; 4],
    pub ink_price_ml: f32,
    pub media: Vec<Media>,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            name: "Ecossolvente 1,80 m".into(),
            printer_model: "Ecossolvente XP600 (AM.CO.ZA)".into(),
            media_width_mm: 1800.0,
            margin_left_mm: 10.0,
            margin_right_mm: 10.0,
            output: Output::Folder(String::new()),
            print_while_rip: true,
            ink_ml: [INK_BOTTLE_ML; 4],
            ink_price_ml: 0.35,
            media: default_media(),
        }
    }
}

impl Setup {
    pub fn printable_mm(&self) -> f32 {
        (self.media_width_mm - self.margin_left_mm - self.margin_right_mm).max(10.0)
    }
}

// ---- layout (nesting, copies and tiling) ---------------------------------------------------

#[derive(Clone, Debug)]
pub struct Placed {
    pub job: u64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Layout {
    pub items: Vec<Placed>,
    pub length_mm: f32,
    pub width_mm: f32,
    pub tiles: u32,
}

/// Shelf layout across the printable width; copies are laid out as separate items.
pub fn layout(jobs: &[&Job], printable: f32) -> Layout {
    let mut items: Vec<(u64, f32, f32)> = Vec::new();
    for j in jobs {
        let (w, h) = j.out_size_mm();
        for _ in 0..j.settings.copies.max(1) {
            items.push((j.id, w, h));
        }
    }
    // a single item wider than the media is tiled
    if items.len() == 1 && items[0].1 > printable {
        let (id, w, h) = items[0];
        let step = (printable - TILE_OVERLAP_MM).max(1.0);
        let tiles = ((w - TILE_OVERLAP_MM) / step).ceil().max(1.0) as u32;
        return Layout {
            items: vec![Placed { job: id, x: 0.0, y: 0.0, w, h }],
            length_mm: h * tiles as f32 + NEST_GAP_MM * (tiles as f32 - 1.0),
            width_mm: w,
            tiles,
        };
    }
    items.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut placed = Vec::new();
    let (mut x, mut y, mut row_h, mut max_w) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (id, w, h) in items {
        if x > 0.0 && x + w > printable {
            x = 0.0;
            y += row_h + NEST_GAP_MM;
            row_h = 0.0;
        }
        placed.push(Placed { job: id, x, y, w, h });
        x += w + NEST_GAP_MM;
        max_w = max_w.max(x - NEST_GAP_MM);
        row_h = row_h.max(h);
    }
    Layout { items: placed, length_mm: y + row_h, width_mm: max_w, tiles: 1 }
}

// ---- estimation ------------------------------------------------------------------------------

pub struct Estimate {
    pub area_m2: f32,
    pub media_m2: f32,
    pub ink_ml: [f32; 4],
    pub minutes: f32,
    pub cost_media: f32,
    pub cost_ink: f32,
}

pub fn estimate(jobs: &[&Job], setup: &Setup) -> Estimate {
    let lay = layout(jobs, setup.printable_mm());
    let media_m2 = setup.media_width_mm / 1000.0 * lay.length_mm / 1000.0;
    let mut area = 0.0;
    let mut ink = [0.0f32; 4];
    for j in jobs {
        let (w, h) = j.out_size_mm();
        let a = w / 1000.0 * h / 1000.0 * j.settings.copies.max(1) as f32;
        area += a;
        let (_, dx, dy) = RESOLUTIONS[j.settings.resolution];
        let dots_m2 = (dx as f32 / 0.0254) * (dy as f32 / 0.0254);
        let level = INK_LEVELS[j.settings.ink_level].1 / 100.0;
        let total: f32 = j.coverage.iter().sum();
        let limit = if total > 0.0 { limit_total_ink(total) / total } else { 1.0 };
        for c in 0..4 {
            ink[c] += a * dots_m2 * j.coverage[c] * limit * DROP_VOLUME_PL * 1e-9 * level;
        }
    }
    let s = jobs.first().map(|j| j.settings.clone()).unwrap_or_default();
    let rate = PRINT_SPEED_BASE / PASSES[s.passes] as f32
        * SPEED_FACTOR[s.speed]
        * DIRECTION_FACTOR[s.direction];
    let minutes = media_m2 / rate.max(0.01) * 60.0;
    let media_price = setup.media.get(s.media).map(|m| m.price_m2).unwrap_or(0.0);
    Estimate {
        area_m2: area,
        media_m2,
        ink_ml: ink,
        minutes,
        cost_media: media_m2 * media_price,
        cost_ink: ink.iter().sum::<f32>() * setup.ink_price_ml,
    }
}

/// Average CMYK coverage of an RGBA picture over white paper.
pub fn coverage_from_rgba(px: &[u8]) -> [f32; 4] {
    let mut sum = [0.0f64; 4];
    let n = (px.len() / 4).max(1);
    for p in px.chunks_exact(4) {
        let a = p[3] as f64 / 255.0;
        let r = 1.0 - a + a * p[0] as f64 / 255.0;
        let g = 1.0 - a + a * p[1] as f64 / 255.0;
        let b = 1.0 - a + a * p[2] as f64 / 255.0;
        let k = 1.0 - r.max(g).max(b);
        if k < 0.999 {
            sum[0] += (1.0 - r - k) / (1.0 - k);
            sum[1] += (1.0 - g - k) / (1.0 - k);
            sum[2] += (1.0 - b - k) / (1.0 - k);
        }
        sum[3] += k;
    }
    [
        (sum[0] / n as f64) as f32,
        (sum[1] / n as f64) as f32,
        (sum[2] / n as f64) as f32,
        (sum[3] / n as f64) as f32,
    ]
}

#[derive(Serialize, Deserialize, Default)]
pub struct SavedState {
    pub jobs: Vec<Job>,
    pub setup: Option<Setup>,
    pub next_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shelf_wraps() {
        let mk = |id, w| Job {
            id,
            name: String::new(),
            path: String::new(),
            art_w_mm: w,
            art_h_mm: 500.0,
            queue: Queue::Hold,
            status: Status::Waiting,
            settings: JobSettings::default(),
            coverage: [0.0; 4],
            preview: None,
            ripped: vec![],
            rip_dpi: 0.0,
            mtime: 0,
            nest: None,
            started: None,
            expected_secs: 0.0,
            demo_progress: None,
        };
        let a = mk(1, 1000.0);
        let b = mk(2, 1000.0);
        let l = layout(&[&a, &b], 1780.0);
        assert_eq!(l.items[1].x, 0.0);
        assert_eq!(l.length_mm, 1010.0);
    }
}
