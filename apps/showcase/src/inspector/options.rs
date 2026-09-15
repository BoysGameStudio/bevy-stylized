//! CLI options; input shape is independent from runtime scene state.
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Options {
    pub(super) asset: PathBuf,
    pub(super) config: Option<PathBuf>,
    pub(super) out: PathBuf,
    pub(super) headless: bool,
    pub(super) once: bool,
    pub(super) exercise_reload: bool,
    pub(super) resolution: u32,
    pub(super) validation: Option<String>,
    pub(super) frames: usize,
}
impl Options {
    pub(super) fn parse(args: &[String]) -> Result<Self, String> {
        let mut o = Self {
            out: "target/stylized/inspector".into(),
            resolution: 1024,
            frames: 120,
            ..Default::default()
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--asset" => o.asset = it.next().ok_or("--asset requires a path")?.into(),
                "--config" => o.config = Some(it.next().ok_or("--config requires a path")?.into()),
                "--out" => o.out = it.next().ok_or("--out requires a path")?.into(),
                "--resolution" => {
                    o.resolution = it
                        .next()
                        .ok_or("--resolution requires pixels")?
                        .parse()
                        .map_err(|_| "invalid resolution")?
                }
                "--headless" => {
                    o.headless = true;
                    o.once = true;
                }
                "--capture-once" => o.once = true,
                "--scan" | "--measure" => {
                    if o.validation.is_some() {
                        return Err("choose only one validation mode".into());
                    }
                    o.validation = Some(a.clone());
                    o.headless = true;
                }
                "--frames" => {
                    o.frames = it
                        .next()
                        .ok_or("--frames requires a count")?
                        .parse()
                        .map_err(|_| "invalid frame count")?
                }
                "--exercise-reload" => o.exercise_reload = true,
                _ => return Err(format!("unknown inspect argument: {a}")),
            }
        }
        if o.asset.as_os_str().is_empty() {
            return Err("usage: inspect --asset COMPILED.glb [--config recipe.json] [--out NEW_DIRECTORY] [--headless|--capture-once] [--exercise-reload]".into());
        }
        if o.validation.is_some()
            && (o.once || o.exercise_reload || !(24..=600).contains(&o.frames))
        {
            return Err(
                "scan/measure requires 24–600 frames and cannot combine with capture/reload modes"
                    .into(),
            );
        }
        if (o.once || o.validation.is_some()) && o.out.exists() {
            return Err("capture output exists; choose a new directory".into());
        }
        Ok(o)
    }
}
