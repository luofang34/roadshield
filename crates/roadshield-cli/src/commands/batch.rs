//! `roadshield batch`: bounded, parallel, deduplicating JSONL renderer.

use std::collections::HashSet;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use clap::ArgMatches;
use roadshield::{DisplayContext, Engine, Rendering, RouteDescriptor, ShieldError};
use roadshield_raster::{AlphaMode, RasterOptions, encode_png, rasterize};
use serde_json::{Value, json};

use super::{context, load_engine, path, print_json};

const CHUNK: usize = 512;

struct Limits {
    max_items: usize,
    max_output: u64,
    deadline: Option<Instant>,
}

enum Outcome {
    Symbol(Box<roadshield::ShieldSymbol>),
    NoShield(Value),
    Error(Value),
}

fn render_line(engine: &Engine, ctx: &DisplayContext, line: &str) -> (Option<String>, Outcome) {
    let route: RouteDescriptor = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return (
                None,
                Outcome::Error(json!({"kind": "invalid_input", "reason": e.to_string()})),
            );
        }
    };
    match engine.render(&route, ctx) {
        Ok(Rendering::Symbol(s)) => (Some(s.semantic_key.clone()), Outcome::Symbol(s)),
        Ok(Rendering::NoShield {
            reason,
            semantic_key,
        }) => (
            Some(semantic_key),
            Outcome::NoShield(serde_json::to_value(reason).unwrap_or(Value::Null)),
        ),
        Err(e) => (None, Outcome::Error(error_value(&e))),
    }
}

fn error_value(e: &ShieldError) -> Value {
    let mut v = serde_json::to_value(e).unwrap_or(Value::Null);
    v["message"] = json!(e.to_string());
    v
}

type Rendered = (usize, Option<String>, Outcome);

fn process_chunk(
    engine: &Engine,
    ctx: &DisplayContext,
    chunk: &[(usize, String)],
    jobs: usize,
) -> Result<Vec<Rendered>> {
    let per = chunk.len().div_ceil(jobs).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = chunk
            .chunks(per)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .map(|(no, l)| {
                            let (k, o) = render_line(engine, ctx, l);
                            (*no, k, o)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut out = Vec::with_capacity(chunk.len());
        for h in handles {
            match h.join() {
                Ok(part) => out.extend(part),
                Err(_) => bail!("a render worker thread terminated abnormally"),
            }
        }
        Ok(out)
    })
}

struct Writer {
    out_dir: PathBuf,
    format: String,
    raster: RasterOptions,
    seen: HashSet<String>,
    written: u64,
    index: std::io::BufWriter<std::fs::File>,
}

impl Writer {
    fn emit(&mut self, line_no: usize, key: Option<String>, outcome: Outcome) -> Result<()> {
        let mut rec = json!({"line": line_no, "semantic_key": key});
        match outcome {
            Outcome::Symbol(s) => {
                let svg_rel = format!("svg/{}.svg", s.semantic_key);
                let png_rel = format!("png/{}@{}x.png", s.semantic_key, self.raster.pixel_ratio);
                if self.seen.insert(s.semantic_key.clone()) {
                    if self.format != "png" {
                        self.write(&svg_rel, s.svg.as_bytes())?;
                    }
                    if self.format != "svg" {
                        let png = encode_png(&rasterize(&s, &self.raster)?)?;
                        self.write(&png_rel, &png)?;
                    }
                }
                rec["status"] = json!("symbol");
                rec["rule"] = json!(s.rule.rule_key);
                rec["width"] = json!(s.width);
                rec["height"] = json!(s.height);
                if self.format != "png" {
                    rec["svg"] = json!(svg_rel);
                }
                if self.format != "svg" {
                    rec["png"] = json!(png_rel);
                }
                if !s.warnings.is_empty() {
                    rec["warnings"] = serde_json::to_value(&s.warnings)?;
                }
            }
            Outcome::NoShield(reason) => {
                rec["status"] = json!("no_shield");
                rec["reason"] = reason;
            }
            Outcome::Error(err) => {
                rec["status"] = json!("error");
                rec["error"] = err;
            }
        }
        serde_json::to_writer(&mut self.index, &rec)?;
        self.index.write_all(b"\n")?;
        Ok(())
    }

    fn write(&mut self, rel: &str, bytes: &[u8]) -> Result<()> {
        self.written += bytes.len() as u64;
        roadshield_import::write_blocking(&self.out_dir.join(rel), bytes)?;
        Ok(())
    }
}

fn open_input(p: &Path, max: u64) -> Result<BufReader<std::fs::File>> {
    let f = std::fs::File::open(p).with_context(|| format!("opening {}", p.display()))?;
    let len = f.metadata()?.len();
    if len > max {
        bail!(
            "{} is {len} bytes, over --max-input-bytes {max}",
            p.display()
        );
    }
    Ok(BufReader::new(f))
}

fn limits(m: &ArgMatches, start: Instant) -> Limits {
    Limits {
        max_items: m
            .get_one::<usize>("max-items")
            .copied()
            .unwrap_or(1_000_000),
        max_output: m
            .get_one::<u64>("max-output-bytes")
            .copied()
            .unwrap_or(u64::MAX),
        deadline: m
            .get_one::<u64>("time-limit")
            .map(|s| start + Duration::from_secs(*s)),
    }
}

fn jobs(m: &ArgMatches) -> usize {
    m.get_one::<usize>("jobs")
        .copied()
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
        .max(1)
}

impl Writer {
    fn create(m: &ArgMatches, out_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&out_dir)
            .with_context(|| format!("creating {}", out_dir.display()))?;
        let index =
            std::fs::File::create(out_dir.join("index.jsonl")).context("creating index.jsonl")?;
        Ok(Writer {
            out_dir,
            format: m
                .get_one::<String>("format")
                .cloned()
                .unwrap_or_else(|| "svg".into()),
            raster: RasterOptions {
                pixel_ratio: m.get_one::<f32>("dpr").copied().unwrap_or(2.0),
                alpha: AlphaMode::Straight,
                ..RasterOptions::default()
            },
            seen: HashSet::new(),
            written: 0,
            index: std::io::BufWriter::new(index),
        })
    }
}

/// Renders a JSONL file of descriptors into `out-dir`.
pub fn run(m: &ArgMatches) -> Result<()> {
    let start = Instant::now();
    let engine = load_engine(m)?;
    let ctx = context(m)?;
    let input = path(m, "input")?;
    let out_dir = path(m, "out-dir")?;
    let limits = limits(m, start);
    let jobs = jobs(m);
    let reader = open_input(
        &input,
        m.get_one::<u64>("max-input-bytes")
            .copied()
            .unwrap_or(u64::MAX),
    )?;
    let mut w = Writer::create(m, out_dir)?;
    let (mut items, mut stopped) = (0usize, None);
    let mut lines = reader
        .lines()
        .enumerate()
        .filter(|(_, l)| l.as_ref().map_or(true, |l| !l.trim().is_empty()));
    'outer: loop {
        let mut chunk = Vec::with_capacity(CHUNK);
        for (no, line) in lines.by_ref().take(CHUNK) {
            chunk.push((no + 1, line.context("reading input")?));
        }
        if chunk.is_empty() {
            break;
        }
        let results = process_chunk(&engine, &ctx, &chunk, jobs)?;
        for (no, key, outcome) in results {
            if items >= limits.max_items {
                stopped = Some("max-items");
                break 'outer;
            }
            w.emit(no, key, outcome)?;
            items += 1;
            if w.written > limits.max_output {
                stopped = Some("max-output-bytes");
                break 'outer;
            }
        }
        if limits.deadline.is_some_and(|d| Instant::now() > d) {
            stopped = Some("time-limit");
            break;
        }
    }
    w.index.flush()?;
    let elapsed = start.elapsed().as_secs_f64();
    #[allow(clippy::cast_precision_loss)]
    let rate = items as f64 / elapsed.max(1e-9);
    print_json(&json!({
        "items": items,
        "unique_symbols": w.seen.len(),
        "bytes_written": w.written,
        "elapsed_s": elapsed,
        "items_per_s": rate,
        "stopped_by": stopped,
    }))
}
