//! ADB (Android Debug Bridge) Rust implementation
//!
//! Provides functionality for communicating with Android devices
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    process::Command,
    sync::Mutex,
    time::Duration,
};

use image::{DynamicImage, codecs::png::PngDecoder};
use tracing::{error, trace};

use utils::{ResponseStatus, read_payload_to_string, read_response_status};

use self::{
    command::{AdbCommand, host_service, local_service},
    host::Host,
    utils::write_request,
};

pub mod command;
pub mod error;
pub mod host;
pub mod utils;

// Re-export commonly used types
pub use error::{AdbError, AdbResult};

#[derive(Debug)]
pub struct DeviceInfo {
    pub serial: String,
    pub info: BTreeMap<String, String>,
}

impl TryFrom<&str> for DeviceInfo {
    type Error = AdbError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Turn "serial\tdevice key1:value1 key2:value2 ..." into a `DeviceInfo`.
        let mut pairs = value.split_whitespace();
        let serial = pairs.next();
        let state = pairs.next();
        if let (Some(serial), Some("device")) = (serial, state) {
            let info: BTreeMap<String, String> = pairs
                .filter_map(|pair| {
                    let mut kv = pair.split(':');
                    if let (Some(k), Some(v), None) = (kv.next(), kv.next(), kv.next()) {
                        Some((k.to_owned(), v.to_owned()))
                    } else {
                        None
                    }
                })
                .collect();

            Ok(DeviceInfo {
                serial: serial.to_owned(),
                info,
            })
        } else {
            Err(AdbError::DeviceInfoParseError(format!(
                "failed to parse device info from {}",
                value
            )))
        }
    }
}

pub struct AdbTcpStream {
    inner: TcpStream,
}

impl AdbTcpStream {
    pub fn connect(socket_addr: SocketAddrV4) -> AdbResult<Self> {
        trace!("connecting to {:?}...", socket_addr);
        let stream = TcpStream::connect(socket_addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let res = Self { inner: stream };
        trace!("connected");
        Ok(res)
    }

    pub fn connect_host() -> AdbResult<Self> {
        Self::connect(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 5037))
    }

    pub fn connect_device<S: AsRef<str>>(serial: S) -> AdbResult<Self> {
        let serial = serial.as_ref();
        let mut stream = Self::connect_host()?;
        stream.execute_command(host_service::Transport::new(serial.to_string()))?;
        Ok(stream)
    }

    pub fn execute_command<T>(&mut self, command: impl AdbCommand<Output = T>) -> AdbResult<T> {
        // TODO: maybe reconnect every time is a good choice?
        // TODO: no, for transport
        trace!("executing command: {:?}...", command.raw_command());
        write_request(self, command.raw_command())?;

        command.handle_response(self)
    }

    pub fn check_response_status(&mut self) -> AdbResult<()> {
        trace!("checking response_status...");
        let status = read_response_status(self)?;
        if let ResponseStatus::Fail = status {
            let reason = read_payload_to_string(self)?;
            error!("response status is FAIL, reason: {}", reason);
            return Err(AdbError::ResponseError(reason));
        }
        trace!("response status is OKAY");
        Ok(())
    }
}

/// Connect to a device using its serial number
///
/// Returns [`AdbError::DeviceNotFound`] if connection fails
pub fn connect<S: AsRef<str>>(serial: S) -> AdbResult<Device> {
    let serial = serial.as_ref();

    let _adb_connect = Command::new("adb")
        .args(["connect", serial])
        .output()
        .map_err(|err| AdbError::DeviceNotFound(format!("{:?}", err)))?;
    // TODO: check stdout of it to find whether the connect is success or not
    // TODO: or, actually the following code can already check?

    let mut host = host::connect_default().expect("failed to connect to adb server");

    let serial = serial.to_string();
    let serials = host
        .devices_long()?
        .iter()
        .map(|device_info| device_info.serial.clone())
        .collect::<Vec<String>>();

    if !serials.contains(&serial) {
        Err(AdbError::DeviceNotFound(serial.clone()))
    } else {
        Ok(Device::new(host, serial))
    }
}

#[allow(unused)]
/// A device that can be used to execute ADB commands
pub struct Device {
    /// The ADB host connection used to access this device
    host: Mutex<Host>,

    /// ADB device serial number
    serial: String,
}

impl Device {
    pub fn new(host: Host, serial: String) -> Self {
        Self {
            host: Mutex::new(host),
            serial,
        }
    }

    pub fn serial(&self) -> String {
        self.serial.clone()
    }

    pub fn input(&self, input: local_service::Input) -> AdbResult<()> {
        self.execute_command_by_socket(input)
    }

    pub fn connect_adb_tcp_stream(&self) -> AdbResult<AdbTcpStream> {
        AdbTcpStream::connect_device(&self.serial).map_err(AdbError::from)
    }

    // pub fn get_screen_size(&self) -> Result<(u32, u32), MyError> {
    //     let screen = self.screencap()?;
    //     Ok((screen.width(), screen.height()))
    // }

    /// Get the raw screencap data in bytes
    pub fn raw_screencap(&self) -> AdbResult<Vec<u8>> {
        // let bytes = self
        //     .execute_command_by_process("exec-out screencap -p")
        //     .expect("failed to screencap");

        // INFO: Using tcp stream to communicate with adb server directly
        // INFO: is about 100ms faster than using process
        let bytes = self
            .execute_command_by_socket(local_service::ScreenCap::new())
            .expect("failed to screencap");
        Ok(bytes)
    }

    /// Get the decoded screencap image
    pub fn screencap(&self) -> AdbResult<image::DynamicImage> {
        let bytes = self.raw_screencap()?;

        let decoder = PngDecoder::new(Cursor::new(bytes)).map_err(AdbError::from)?;
        let image = DynamicImage::from_decoder(decoder).map_err(AdbError::from)?;
        Ok(image)
    }

    /// `adb -s <self.serial> <command>`
    pub fn execute_command_by_process(&self, command: &str) -> AdbResult<Vec<u8>> {
        let mut args = vec!["-s", self.serial.as_str()];
        args.extend(command.split_whitespace().collect::<Vec<&str>>());

        let res = Command::new("adb")
            .args(args)
            .output()
            .map_err(AdbError::from)?
            .stdout;
        Ok(res)
    }

    pub fn execute_command_by_socket<T>(
        &self,
        command: impl AdbCommand<Output = T>,
    ) -> AdbResult<T> {
        let mut adb_tcp_stream = self.connect_adb_tcp_stream()?;
        adb_tcp_stream
            .execute_command(command)
            .map_err(AdbError::from)
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;

    use super::*;
    use crate::command::local_service;

    fn device() -> Device {
        connect("192.168.1.3:40919").unwrap()
    }

    #[test]
    fn test_connect() -> AdbResult<()> {
        let _device = device();
        Ok(())
    }

    #[test]
    fn test_screencap() {
        // by process cost: 938.7667ms, 3066595
        // by socket cost: 841.6327ms, 3069330
        let device = device();

        let start = Instant::now();
        let bytes = device
            .execute_command_by_process("exec-out screencap -p")
            .unwrap();
        println!("by process cost: {:?}, {}", start.elapsed(), bytes.len());

        let start = Instant::now();
        let bytes2 = device
            .execute_command_by_socket(local_service::ScreenCap::new())
            .unwrap();
        println!("by socket cost: {:?}, {}", start.elapsed(), bytes2.len());

        // assert_eq!(bytes, bytes2);
    }
}

impl Read for AdbTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for AdbTcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
