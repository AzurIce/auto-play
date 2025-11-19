pub mod task;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// 对应 `resource/resource.toml`
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(with = "time::serde::rfc3339")]
    pub last_updated: OffsetDateTime,
}

impl PartialEq for Manifest {
    fn eq(&self, other: &Self) -> bool {
        self.last_updated == other.last_updated
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_serialize() {
        let manifest = Manifest {
            last_updated: OffsetDateTime::now_utc(),
        };
        let toml = toml::to_string_pretty(&manifest).unwrap();
        println!("{}", toml);
    }

    //     #[test]
    //     fn test_deserialize() {
    //         let json = r#"{
    //     "last_updated": "2024-09-18T22:52:42.3862364+08:00"
    // }"#;

    //         println!("{}", json);
    //         let version: Manifest = serde_json::from_str(json).unwrap();
    //         println!("{:?}", version);
    //     }
}
