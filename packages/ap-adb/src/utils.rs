use std::{
    io::{Read, Write},
    process::Command,
    str::FromStr,
};

use super::error::{AdbError, AdbResult};

pub fn execute_adb_command(serial: &str, command: &str) -> AdbResult<Vec<u8>> {
    let mut args = vec!["-s", serial];
    args.extend(command.split_whitespace().collect::<Vec<&str>>());

    let res = Command::new("adb")
        .args(args)
        .output()
        .map_err(AdbError::from)?
        .stdout;
    Ok(res)
}

// Streaming

pub fn read_exact<T: Read>(source: &mut T, len: usize) -> AdbResult<Vec<u8>> {
    let mut buf = [0; 65536];
    source.read_exact(&mut buf[..len]).map_err(AdbError::from)?;
    Ok(buf[..len].to_vec())
}

pub fn read_exact_to_string<T: Read>(source: &mut T, len: usize) -> AdbResult<String> {
    let bytes = read_exact(source, len)?;
    let s = std::str::from_utf8(&bytes).map_err(AdbError::from)?;
    Ok(s.to_string())
}

pub fn read_to_end<T: Read>(source: &mut T) -> AdbResult<Vec<u8>> {
    let mut response = Vec::new();
    source.read_to_end(&mut response).map_err(AdbError::from)?;
    Ok(response)
}

pub fn read_to_end_to_string<T: Read>(source: &mut T) -> AdbResult<String> {
    let bytes = read_to_end(source)?;
    let s = std::str::from_utf8(&bytes).map_err(AdbError::from)?;
    Ok(s.to_string())
}

// Following are more utilized things

pub fn read_payload_len<T: Read>(source: &mut T) -> AdbResult<usize> {
    let len = read_exact_to_string(source, 4)?;
    let len = usize::from_str_radix(&len, 16).map_err(AdbError::from)?;
    Ok(len)
}

pub fn read_payload<T: Read>(source: &mut T) -> AdbResult<Vec<u8>> {
    let len = read_payload_len(source)?;
    let bytes = read_exact(source, len)?;
    Ok(bytes)
}

pub fn read_payload_to_string<T: Read>(source: &mut T) -> AdbResult<String> {
    let bytes = read_payload(source)?;
    let s = std::str::from_utf8(&bytes).map_err(AdbError::from)?;
    Ok(s.to_string())
}

#[derive(Debug)]
pub enum ResponseStatus {
    Okay,
    Fail,
}

impl FromStr for ResponseStatus {
    type Err = AdbError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OKAY" => Ok(Self::Okay),
            "FAIL" => Ok(Self::Fail),
            _ => Err(AdbError::UnknownResponseStatus(s.to_string())),
        }
    }
}

pub fn read_response_status<T: Read>(source: &mut T) -> AdbResult<ResponseStatus> {
    let status = read_exact_to_string(source, 4)?;
    let status = ResponseStatus::from_str(&status)?;
    Ok(status)
}

pub fn write_request<T: Write>(target: &mut T, request: String) -> AdbResult<()> {
    target
        .write_all(format!("{:04x}{}", request.len(), request).as_bytes())
        .map_err(AdbError::from)
}
