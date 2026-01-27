use std::time::Duration;

use crate::{
    AdbTcpStream,
    error::AdbResult,
    utils::{read_to_end, read_to_end_to_string},
};

use super::AdbCommand;

/// shell:command
///
/// command is something like "cmd arg1 arg2 ..."
pub struct ShellCommand {
    command: String,
}

impl ShellCommand {
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl AdbCommand for ShellCommand {
    type Output = String;

    fn raw_command(&self) -> String {
        format!("shell:{}", self.command)
    }

    fn handle_response(&self, stream: &mut AdbTcpStream) -> AdbResult<Self::Output> {
        stream.check_response_status()?;
        read_to_end_to_string(stream)
    }
}

/// Png decoded screencap
///
/// `shell:screencap -p`
pub struct ScreenCapPng;

impl ScreenCapPng {
    pub fn new() -> Self {
        Self
    }
}

impl AdbCommand for ScreenCapPng {
    type Output = Vec<u8>;

    fn raw_command(&self) -> String {
        "shell:screencap -p".to_string()
    }

    fn handle_response(&self, stream: &mut AdbTcpStream) -> AdbResult<Self::Output> {
        stream.check_response_status()?;
        read_to_end(stream)
    }
}

/// Raw screencap (unencoded framebuffer data)
///
/// Returns raw pixel data with a 12-byte header, significantly faster than PNG encoding.
///
/// ## Command
/// `shell:screencap`
///
/// ## Data Format
/// ```text
/// Header (12 bytes, Little Endian):
///   [0..4]   Width  (u32)
///   [4..8]   Height (u32)
///   [8..12]  Format (u32)
///            - 1 = RGBA_8888 (Red, Green, Blue, Alpha)
///
/// Pixel Data (width * height * 4 bytes):
///   Raw pixel bytes
/// ```
pub struct ScreenCapRaw;

impl ScreenCapRaw {
    pub fn new() -> Self {
        Self
    }
}

impl AdbCommand for ScreenCapRaw {
    type Output = (u32, u32, Vec<u8>);

    fn raw_command(&self) -> String {
        "shell:screencap".to_string()
    }

    fn handle_response(&self, stream: &mut AdbTcpStream) -> AdbResult<Self::Output> {
        stream.check_response_status()?;
        let data = read_to_end(stream)?;
        let width = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let format = u32::from_le_bytes(data[8..12].try_into().unwrap());
        assert_eq!(format, 1);
        let pixel_data = data[12..].to_vec();
        Ok((width, height, pixel_data))
    }
}

pub enum Input {
    /// shell:input swipe x1 y1 x2 y2 duration
    Swipe {
        p1: (u32, u32),
        p2: (i32, i32),
        duration: Duration,
    },
    /// .0 is keycode
    ///
    /// shell:input keyevent <keycode>
    Keyevent(String),
}

impl AdbCommand for Input {
    type Output = ();

    fn raw_command(&self) -> String {
        match self {
            Input::Swipe { p1, p2, duration } => {
                format!(
                    "shell:input swipe {} {} {} {} {}",
                    p1.0,
                    p1.1,
                    p2.0,
                    p2.1,
                    duration.as_millis()
                )
            }
            Input::Keyevent(keycode) => format!("shell:input keyevent {}", keycode),
        }
    }

    fn handle_response(&self, stream: &mut AdbTcpStream) -> AdbResult<Self::Output> {
        stream.check_response_status()
    }
}

#[cfg(test)]
mod test {
    use crate::host;

    use super::{ScreenCapPng, ShellCommand};

    #[test]
    fn test_screencap() {
        let mut host = host::connect_default().unwrap();
        let res = host
            .execute_local_command("127.0.0.1:16384".to_string(), ScreenCapPng::new())
            .unwrap();
        println!("{}", res.len())
    }

    #[test]
    fn test_minitouch() {
        let mut host = host::connect_default().unwrap();
        let res = host
            .execute_local_command(
                "127.0.0.1:16384".to_string(),
                ShellCommand::new("/data/local/tmp/minitouch -h".to_string()),
            )
            .unwrap();
        println!("{res}")
    }
}
