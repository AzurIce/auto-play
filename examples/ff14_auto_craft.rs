//! FF14 Auto-Crafting
//!
//! Automates the crafting loop in FF14:
//! 1. Detect "开始制作作业" button → click it
//! 2. Detect "作业中止" (crafting in progress) → press R to run macro
//! 3. Wait for crafting to finish (back to step 1)
//! 4. Repeat for N times
//!
//! Usage:
//!   cargo run --example ff14_auto_craft --features windows
//!   cargo run --example ff14_auto_craft --features windows -- --count 30

use auto_play::{AutoPlay, ControllerTrait, DynamicImage, MatcherOptions, WindowsController};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};
use tracing::{info, warn};

const WINDOW_TITLE: &str = "最终幻想XIV";
const CRAFT_START_TIMEOUT: Duration = Duration::from_secs(5);
const CRAFT_FINISH_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq)]
enum CraftState {
    Ready,
    InProgress,
    Unknown,
}

struct FF14Crafter {
    ap: AutoPlay,
    tpl_start: DynamicImage,
    tpl_stop: DynamicImage,
    options: MatcherOptions,
}

impl FF14Crafter {
    fn new() -> anyhow::Result<Self> {
        info!("连接窗口 '{WINDOW_TITLE}'...");
        let controller = WindowsController::from_window_title(WINDOW_TITLE)?;
        let (w, h) = controller.screen_size();
        info!("已连接: {w}x{h}");

        let ap = AutoPlay::new(controller);
        let tpl_start = image::open("assets/start_crafting.png")?;
        let tpl_stop = image::open("assets/stop_crafting.png")?;
        info!(
            "模板已加载: start={}x{}, stop={}x{}",
            tpl_start.width(),
            tpl_start.height(),
            tpl_stop.width(),
            tpl_stop.height()
        );

        Ok(Self {
            ap,
            tpl_start,
            tpl_stop,
            options: MatcherOptions::default(),
        })
    }

    fn win(&self) -> &WindowsController {
        self.ap.controller_ref::<WindowsController>().unwrap()
    }

    fn detect_state(&self) -> anyhow::Result<CraftState> {
        if self.ap.find_image(&self.tpl_stop, &self.options)?.is_some() {
            return Ok(CraftState::InProgress);
        }
        let strict = MatcherOptions::default().with_threshold(0.1);
        if self.ap.find_image(&self.tpl_start, &strict)?.is_some() {
            return Ok(CraftState::Ready);
        }
        Ok(CraftState::Unknown)
    }

    fn wait_for_state(
        &self,
        target: CraftState,
        timeout: Duration,
        pb: &ProgressBar,
        msg: &str,
    ) -> anyhow::Result<bool> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            pb.set_message(format!("{msg} ({:.0}s)", start.elapsed().as_secs_f32()));
            if self.detect_state()? == target {
                return Ok(true);
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        Ok(false)
    }

    fn craft_once(&self, pb: &ProgressBar) -> anyhow::Result<bool> {
        pb.set_message("寻找 '开始制作作业'...");
        let Some(rect) = self.ap.find_image(&self.tpl_start, &self.options)? else {
            warn!("'开始制作作业' 未找到");
            return Ok(false);
        };
        let click_x = rect.x + rect.width / 2;
        let click_y = rect.y + rect.height / 2;
        self.win().focus_click(click_x, click_y)?;
        info!("点击 ({click_x}, {click_y})");

        std::thread::sleep(Duration::from_millis(500));
        if !self.wait_for_state(
            CraftState::InProgress,
            CRAFT_START_TIMEOUT,
            pb,
            "等待制作窗口",
        )? {
            warn!("制作窗口未出现");
            return Ok(false);
        }

        std::thread::sleep(Duration::from_millis(300));
        pb.set_message("执行宏 (R)...");
        self.win()
            .focus_press(auto_play::controller::Key::Unicode('r'))?;

        if !self.wait_for_state(CraftState::Ready, CRAFT_FINISH_TIMEOUT, pb, "制作中")? {
            warn!("制作超时");
            return Ok(false);
        }

        Ok(true)
    }

    fn run(&self, count: u32) -> anyhow::Result<()> {
        let state = self.detect_state()?;
        info!("当前状态: {state:?}");
        if state != CraftState::Ready {
            warn!("请先打开制作笔记并选择配方");
            return Ok(());
        }

        let pb = ProgressBar::new(count as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} {msg} ({elapsed})",
            )?
            .progress_chars("█▓░"),
        );
        pb.set_message("准备中...");

        let mut success = 0u32;
        for i in 1..=count {
            let start = Instant::now();
            match self.craft_once(&pb) {
                Ok(true) => {
                    success += 1;
                    pb.inc(1);
                    let elapsed = start.elapsed().as_secs_f32();
                    pb.set_message(format!("✓ {success}成功 | 上次 {elapsed:.1}s"));
                    std::thread::sleep(Duration::from_millis(500));
                }
                Ok(false) => {
                    pb.set_message(format!("✗ 第{i}次失败，停止"));
                    pb.abandon();
                    return Ok(());
                }
                Err(e) => {
                    pb.set_message(format!("✗ 第{i}次出错: {e}"));
                    pb.abandon();
                    return Err(e);
                }
            }
        }

        pb.finish_with_message(format!("完成: {success}/{count} 成功"));
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    use tracing_indicatif::IndicatifLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let indicatif_layer = IndicatifLayer::new();
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ff14_auto_craft=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif_layer.get_stderr_writer())
                .without_time()
                .with_target(false),
        )
        .with(indicatif_layer)
        .init();

    let count = std::env::args()
        .skip_while(|a| a != "--count")
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(10);

    let crafter = FF14Crafter::new()?;
    crafter.run(count)
}
