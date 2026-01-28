use std::{
    io::{BufRead, Write},
    process::{Child, ChildStdin, Command, Stdio},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Context;
use color_print::cformat;
use tempfile::NamedTempFile;
use tracing::{debug, info, trace};

use ap_adb::{Device, command::local_service::ShellCommand, utils::execute_adb_command};

const MAATOUCH: &[u8] = include_bytes!("./maatouch");

use super::App;

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// After initialized, hold a child-stdin to write commands to maatouch
/// If disconnected during using, it should be reconstructed
pub struct MaaTouch {
    child: Child,
    child_in: ChildStdin,
    state: MaaTouchState,
    // cmd_tx: async_channel::Sender<Cmd>,
}

impl Drop for MaaTouch {
    fn drop(&mut self) {
        // Note that the commands are not done immediately, it will take some time to execute.
        // Before that we should not drop the controller, or the maatouch process will be killed.
        //
        // Ideally, maatouch should accept a "q" command to quit, and we wait for the process to quit here.
        // Now we just wait for a short time to ensure the commands are executed.
        thread::sleep(Duration::from_millis(100));
        self.child.kill().unwrap()
    }
}

#[allow(unused)]
#[derive(Default)]
pub struct MaaTouchState {
    flip_xy: bool,
    max_contact: u32,
    max_x: u32, // 横屏的 x!
    max_y: u32,
    max_pressure: u32,
}

impl App for MaaTouch {
    fn check(device: &Device) -> anyhow::Result<()> {
        let mut device_adb_stream = device
            .connect_adb_tcp_stream()
            .map_err(|err| anyhow::anyhow!("maatouch connect AdbTcpStream failed :{err}"))?;

        info!("[Minitouch]: checking maatouch...");
        let res = device_adb_stream
            .execute_command(ShellCommand::new(
                "file /data/local/tmp/maatouch".to_string(),
            ))
            .map_err(|err| anyhow::anyhow!("maatouch test failed: {err}"))?;
        info!("[Minitouch]: test output: {res}");

        // [Minitouch]: test output: /data/local/tmp/maatouch: Zip archive data
        if !res.contains("Zip archive data") {
            anyhow::bail!("maatouch exec failed");
        }
        Ok(())
    }

    fn push(device: &Device) -> anyhow::Result<()> {
        // let abi = device
        //     .get_abi()
        //     .map_err(|err| anyhow::anyhow!("get abi failed: {err}"))?;
        // let maatouch_bytes = match abi.as_str() {
        //     "armeabi-v7a" => MINITOUCH_ARM,
        //     "arm64-v8a" => MINITOUCH_ARM_64,
        //     "x86" => MINITOUCH_X86,
        //     "x86_64" => MINITOUCH_X86_64,
        //     _ => anyhow::bail!("unsupported abi: {}", abi),
        // };
        let mut tmpfile = NamedTempFile::new().context("failed to create tempfile")?;
        tmpfile
            .write_all(MAATOUCH)
            .context("failed to write maatouch to tempfile")?;

        info!(
            "{}",
            cformat!("<dim>[Minitouch]: pushing maatouch to device...</dim>")
        );
        let cmd = format!("push {} /data/local/tmp", tmpfile.path().to_str().unwrap());
        let res = execute_adb_command(&device.serial(), &cmd)
            .map_err(|err| anyhow::anyhow!("maatouch push failed: {:?}", err))?;
        info!("{:?}", String::from_utf8(res));

        info!(
            "{}",
            cformat!(
                "<dim>[Minitouch]: renaming {:?} to maatouch...</dim>",
                tmpfile.path().file_name()
            )
        );
        let cmd = format!(
            "shell mv /data/local/tmp/{} /data/local/tmp/maatouch",
            tmpfile.path().file_name().unwrap().to_str().unwrap()
        );
        let res = execute_adb_command(&device.serial(), &cmd)
            .map_err(|err| anyhow::anyhow!("maatouch rename failed: {:?}", err))?;
        info!("<dim>[Minitouch]: {:?}</dim>", String::from_utf8(res));

        info!(
            "{}",
            cformat!("<dim>[Minitouch]: adding execute permission to maatouch...</dim>")
        );
        let res = execute_adb_command(&device.serial(), "shell chmod +x /data/local/tmp/maatouch")
            .map_err(|err| anyhow::anyhow!("maatouch push failed: {:?}", err))?;
        info!("{:?}", String::from_utf8(res));
        // sleep(Duration::from_millis(200));
        Ok(())
    }

    fn build(device: &Device) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        info!(
            "{}",
            cformat!("<dim>[Minitouch]: spawning maatouch...</dim>")
        );
        let mut child = Command::new("adb")
            .args(vec![
                "-s",
                device.serial().as_str(),
                "shell",
                "app_process -Djava.class.path=/data/local/tmp/maatouch /data/local/tmp com.shxyke.MaaTouch.App",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("failed to spawn maatouch")?;
        sleep(Duration::from_secs_f32(0.5));

        let child_in = child
            .stdin
            .take()
            .ok_or(anyhow::anyhow!("cannot get stdin of maatouch"))?;
        let child_out = child
            .stdout
            .take()
            .ok_or(anyhow::anyhow!("cannot get stdout of maatouch"))?;

        // let (cmd_tx, cmd_rx) = async_channel::unbounded::<Cmd>();

        let mut state = MaaTouchState::default();
        // read info
        debug!("reading maatouch info...");
        let mut reader = std::io::BufReader::new(child_out);
        loop {
            let mut buf = String::new();
            match reader.read_line(&mut buf) {
                Err(err) => {
                    info!(
                        "{}",
                        cformat!("<dim>[Minitouch]: read error: {}</dim>", err)
                    );
                    anyhow::bail!("failed to read maatouch info: {}", err);
                }
                Ok(sz) => {
                    trace!("readed {sz} len: {buf:?}");
                    if sz == 0 {
                        // println!("readed Ok(0)");
                        continue;
                    }
                    buf = buf
                        .replace("\r\n", "\n")
                        .strip_suffix("\n")
                        .unwrap()
                        .to_string();
                    println!("readed info: {}", buf);
                    if buf.starts_with('^') {
                        let params = &buf.split(' ').skip(1).collect::<Vec<&str>>();
                        let max_contact = u32::from_str_radix(params[0], 10).unwrap();
                        let max_size1 = u32::from_str_radix(params[1], 10).unwrap();
                        let max_size2 = u32::from_str_radix(params[2], 10).unwrap();
                        let max_pressure = u32::from_str_radix(params[3], 10).unwrap();

                        let mut flip_xy = false;
                        let (max_x, max_y) = if max_size1 > max_size2 {
                            (max_size1, max_size2)
                        } else {
                            flip_xy = true;
                            (max_size2, max_size1)
                        };

                        state = MaaTouchState {
                            flip_xy,
                            max_contact,
                            max_x,
                            max_y,
                            max_pressure,
                        };
                        // maatouch_state.flip_xy = flip_xy;
                        // maatouch_state.max_contact = max_contact;
                        // maatouch_state.max_x = max_x;
                        // maatouch_state.max_y = max_y;
                        // maatouch_state.max_pressure = max_pressure;
                        info!(
                            "{}",
                            cformat!(
                                "<dim>[MaaTouch]: {} {}x{} {} flip: {}</dim>",
                                max_contact,
                                max_x,
                                max_y,
                                max_pressure,
                                flip_xy,
                            ),
                        );
                    } else if buf.starts_with('$') {
                        break;
                    }
                }
            }
        }

        info!(
            "{}",
            cformat!("<dim>[Minitouch]: maatouch initialized</dim>")
        );
        Ok(MaaTouch {
            child,
            child_in,
            state,
        })
    }
}

const SWIPE_DELAY_MS: u32 = 5;
const CLICK_DELAY_MS: u32 = 50;

impl MaaTouch {
    fn write_command(&mut self, command: &str) -> anyhow::Result<()> {
        trace!("[MaaTouch]: writing command {:?}", command);
        let mut command = command.to_string();
        if !command.ends_with('\n') {
            command.push('\n');
        }
        self.child_in
            .write_all(command.as_bytes())
            .context("failed to write command")
    }

    pub fn commit(&mut self) -> anyhow::Result<()> {
        self.write_command("c")
    }

    pub fn reset(&mut self) -> anyhow::Result<()> {
        self.write_command("r")
    }

    pub fn down(&mut self, contact: u32, x: u32, y: u32, pressure: u32) -> anyhow::Result<()> {
        // On MuMu emulator, the x-y is flipped and the y is also flipped (???)
        let (x, y) = if self.state.flip_xy {
            (self.state.max_y.saturating_add_signed(-(y as i32)), x)
        } else {
            (x, y)
        };
        // let y = self.state.max_y.saturating_add_signed(-(y as i32));
        self.write_command(format!("d {contact} {x} {y} {pressure}").as_str())
    }

    pub fn mv(&mut self, contact: u32, x: i32, y: i32, pressure: u32) -> anyhow::Result<()> {
        // On MuMu emulator, the x-y is flipped and the y is also flipped (???)
        let (x, y) = if self.state.flip_xy {
            (self.state.max_y as i32 - y, x)
        } else {
            (x, y)
        };
        // let y = self.state.max_y as i32 - y;
        self.write_command(format!("m {contact} {x} {y} {pressure}").as_str())
    }

    pub fn up(&mut self, contact: u32) -> anyhow::Result<()> {
        self.write_command(format!("u {contact}").as_str())
    }

    pub fn wait(&mut self, duration: Duration) -> anyhow::Result<()> {
        // self.write_command(format!("w {}", duration.as_millis()).as_str())
        thread::sleep(duration);
        Ok(())
    }

    pub fn click(&mut self, x: u32, y: u32) -> anyhow::Result<()> {
        debug!("[MaaTouch/click]: click at {x},{y}");
        self.down(0, x, y, self.state.max_pressure)?;
        self.commit()?;
        self.wait(Duration::from_millis(CLICK_DELAY_MS as u64))?;
        self.up(0)?;
        self.commit()?;
        Ok(())
    }

    pub fn swipe(
        &mut self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        debug!(
            "[MaaTouch/swipe]: swipe from {start:?} to {end:?} for {duration:?} with slope in/out {slope_in}/{slope_out}"
        );
        self.down(0, start.0, start.1, self.state.max_pressure)?;
        self.commit()?;

        // 三次样条插值
        let cubic_spline = |slope_0: f32, slope_1: f32, t: f32| -> f32 {
            let a = slope_0;
            let b = -(2.0 * slope_0 + slope_1 - 3.0);
            let c = -(-slope_0 - slope_1 + 2.0);
            a * t + b * t.powf(2.0) + c * t.powf(3.0)
        };

        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

        for t in (SWIPE_DELAY_MS..duration.as_millis() as u32).step_by(SWIPE_DELAY_MS as usize) {
            let progress =
                cubic_spline(slope_in, slope_out, t as f32 / duration.as_millis() as f32);
            let progress = progress.min(1.0).max(0.0);
            // info!("{}", progress);
            // println!("{progress}");
            let cur_x = lerp(start.0 as f32, end.0 as f32, progress) as i32;
            let cur_y = lerp(start.1 as f32, end.1 as f32, progress) as i32;
            // println!("{cur_x} {cur_y}");
            self.mv(0, cur_x as i32, cur_y as i32, self.state.max_pressure)?;
            self.commit()?;
            self.wait(Duration::from_millis(SWIPE_DELAY_MS as u64))?;
            thread::sleep(Duration::from_millis(SWIPE_DELAY_MS as u64));
        }

        // self.mv(0, end.0, end.1, 0)?;
        self.wait(Duration::from_millis(200))?;
        self.commit()?;
        thread::sleep(Duration::from_millis(200));
        self.up(0)?;
        self.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::tests::init_tracing_subscriber;
    use ap_adb::connect;

    use super::*;

    #[test]
    fn test_maatoucher() {
        init_tracing_subscriber();

        info!("test_maatoucher");
        // mumu
        let device = connect("127.0.0.1:16384").unwrap();
        let mut toucher = MaaTouch::build(&device).unwrap();
        // toucher.click(10, 10).unwrap();
        // toucher.click(100, 100).unwrap();
        toucher.click(822, 762).unwrap();
        thread::sleep(Duration::from_secs_f32(2.0));

        // // leidian
        // let device = connect("emulator-5554").unwrap();
        // let mut toucher = MaaTouch::init(&device).unwrap();
        // toucher.click(822, 762).unwrap();
        // thread::sleep(Duration::from_secs_f32(2.0));
    }

    #[test]
    fn test_slowly_swipe() {
        init_tracing_subscriber();
        // let device = connect("127.0.0.1:16384").unwrap();
        let device = connect("emulator-5554").unwrap();
        let mut toucher = MaaTouch::build(&device).unwrap();
        toucher
            .swipe(
                (1780, 400),
                (400, 400),
                Duration::from_millis(400),
                2.0,
                0.0,
            )
            .unwrap();
        thread::sleep(Duration::from_secs_f32(2.0))
    }
}
