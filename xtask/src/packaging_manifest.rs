use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::packaging::{PACKAGING_MANIFEST_SCHEMA_VERSION, PackagingSignatureState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactManifestInput {
    path: PathBuf,
    target: String,
    channel: String,
    signature_state: PackagingSignatureState,
    build_source: String,
}

impl ArtifactManifestInput {
    #[must_use]
    pub fn new(
        path: impl AsRef<Path>,
        target: impl Into<String>,
        channel: impl Into<String>,
        signature_state: PackagingSignatureState,
        build_source: impl Into<String>,
    ) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            target: target.into(),
            channel: channel.into(),
            signature_state,
            build_source: build_source.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedArtifactManifest {
    schema_version: u32,
    entries: Vec<GeneratedArtifactManifestEntry>,
}

impl GeneratedArtifactManifest {
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub fn entries(&self) -> &[GeneratedArtifactManifestEntry] {
        &self.entries
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let entries = self
            .entries
            .iter()
            .map(GeneratedArtifactManifestEntry::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schemaVersion\":{},\"entries\":[{}]}}",
            self.schema_version, entries
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedArtifactManifestEntry {
    file_name: String,
    target: String,
    channel: String,
    sha256: String,
    size_bytes: u64,
    signature_state: PackagingSignatureState,
    build_source: String,
}

impl GeneratedArtifactManifestEntry {
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"fileName\":\"{}\",\"target\":\"{}\",\"channel\":\"{}\",\"sha256\":\"{}\",\"sizeBytes\":{},\"signatureState\":\"{}\",\"buildSource\":\"{}\"}}",
            escape_json(&self.file_name),
            escape_json(&self.target),
            escape_json(&self.channel),
            self.sha256,
            self.size_bytes,
            self.signature_state.as_str(),
            escape_json(&self.build_source)
        )
    }
}

pub struct ArtifactManifestGenerator;

impl ArtifactManifestGenerator {
    pub fn generate(
        inputs: &[ArtifactManifestInput],
    ) -> Result<GeneratedArtifactManifest, ArtifactManifestError> {
        let mut entries = Vec::with_capacity(inputs.len());
        for input in inputs {
            let bytes =
                fs::read(&input.path).map_err(|_| ArtifactManifestError::MissingArtifact {
                    path: input.path.display().to_string(),
                })?;
            let metadata =
                fs::metadata(&input.path).map_err(|_| ArtifactManifestError::MissingArtifact {
                    path: input.path.display().to_string(),
                })?;
            let file_name = input
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| ArtifactManifestError::InvalidFileName {
                    path: input.path.display().to_string(),
                })?;
            entries.push(GeneratedArtifactManifestEntry {
                file_name: file_name.to_string(),
                target: input.target.clone(),
                channel: input.channel.clone(),
                sha256: sha256_hex(&bytes),
                size_bytes: metadata.len(),
                signature_state: input.signature_state,
                build_source: input.build_source.clone(),
            });
        }
        entries.sort_by(|left, right| left.file_name.cmp(&right.file_name));
        Ok(GeneratedArtifactManifest {
            schema_version: PACKAGING_MANIFEST_SCHEMA_VERSION,
            entries,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactManifestError {
    MissingArtifact { path: String },
    InvalidFileName { path: String },
}

impl fmt::Display for ArtifactManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArtifact { path } => write!(formatter, "missing artifact: {path}"),
            Self::InvalidFileName { path } => {
                write!(formatter, "invalid artifact file name: {path}")
            }
        }
    }
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut state = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (bytes.len() as u64) * 8;
    let mut message = bytes.to_vec();
    message.push(0x80);
    while (message.len() % 64) != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in message.chunks(64) {
        compress(&mut state, chunk);
    }
    state.iter().map(|word| format!("{word:08x}")).collect()
}

fn compress(state: &mut [u32; 8], chunk: &[u8]) {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut schedule = [0_u32; 64];
    for (index, word) in schedule.iter_mut().take(16).enumerate() {
        let offset = index * 4;
        *word = u32::from_be_bytes([
            chunk[offset],
            chunk[offset + 1],
            chunk[offset + 2],
            chunk[offset + 3],
        ]);
    }
    for index in 16..64 {
        let s0 = small_sigma0(schedule[index - 15]);
        let s1 = small_sigma1(schedule[index - 2]);
        schedule[index] = schedule[index - 16]
            .wrapping_add(s0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for index in 0..64 {
        let temp1 = h
            .wrapping_add(big_sigma1(e))
            .wrapping_add(ch(e, f, g))
            .wrapping_add(K[index])
            .wrapping_add(schedule[index]);
        let temp2 = big_sigma0(a).wrapping_add(maj(a, b, c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}

const fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

const fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

const fn big_sigma0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

const fn big_sigma1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

const fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

const fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}
