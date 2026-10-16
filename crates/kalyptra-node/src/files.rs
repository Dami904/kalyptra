//! Node identity, key share and JSON files.
//!
//! Secret files (a node's identity keys and its decryption share) are written with owner-only
//! permissions on Unix. Anyone holding `t` decryption shares can open every sealed order.

use std::fs;
use std::path::{Path, PathBuf};

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::SigningKey;
use kalyptra_tse::{GroupKey, NodeKey};
use rand_core::OsRng;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::wire::{hx, unhex_n, GroupJ, RosterNodeJ, WireResult};

pub fn read_json<T: DeserializeOwned>(path: &Path) -> WireResult<T> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

/// Like [`read_json`], but a file that does not exist is the default value. A file that exists
/// and cannot be read or parsed is still an error: it is never silently treated as empty.
pub fn read_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> WireResult<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    read_json(path)
}

pub fn write_json<T: Serialize>(path: &Path, value: &T, secret: bool) -> WireResult<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    text.push('\n');
    // Write a sibling file, flush it, then rename it over the target, so a crash mid-write
    // leaves the old file or the new one, never half of one.
    let tmp = path.with_extension("json.tmp");
    let written = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut f = create_private(&tmp, secret)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()
    })();
    text.zeroize();
    written.map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    if secret {
        restrict(&tmp)?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Create (or truncate) a file. A secret one is owner-only from the moment it exists, so its
/// contents are never readable by others even briefly.
#[cfg(unix)]
fn create_private(path: &Path, secret: bool) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    if secret {
        o.mode(0o600);
    }
    o.open(path)
}

#[cfg(not(unix))]
fn create_private(path: &Path, _secret: bool) -> std::io::Result<fs::File> {
    fs::File::create(path)
}

#[cfg(unix)]
fn restrict(path: &Path) -> WireResult<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("cannot restrict {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> WireResult<()> {
    Ok(())
}

pub fn key_path(dir: &Path, id: u16) -> PathBuf {
    dir.join(format!("node-{id}.key.json"))
}

pub fn pub_path(dir: &Path, id: u16) -> PathBuf {
    dir.join(format!("node-{id}.pub.json"))
}

pub fn share_path(dir: &Path, id: u16) -> PathBuf {
    dir.join(format!("node-{id}.share.json"))
}

#[derive(Serialize, Deserialize)]
struct KeyFileJ {
    id: u16,
    url: String,
    sign_sk: String,
    dkg_sk: String,
}

/// A node's long-term identity: the key that signs its statements, and the key that receives
/// its private key-generation messages. Both are separate from the decryption share.
pub struct NodeIdentity {
    pub id: u16,
    pub url: String,
    pub sign: SigningKey,
    pub dkg: Scalar,
}

impl NodeIdentity {
    pub fn generate(id: u16, url: &str) -> Self {
        NodeIdentity {
            id,
            url: url.trim_end_matches('/').to_string(),
            sign: SigningKey::generate(&mut OsRng),
            dkg: Scalar::random(&mut OsRng),
        }
    }

    pub fn public_entry(&self) -> RosterNodeJ {
        RosterNodeJ {
            id: self.id,
            url: self.url.clone(),
            sign_pk: hx(self.sign.verifying_key().as_bytes()),
            dkg_pk: hx((G * self.dkg).compress().as_bytes()),
        }
    }

    pub fn save(&self, path: &Path) -> WireResult<()> {
        let mut f = KeyFileJ {
            id: self.id,
            url: self.url.clone(),
            sign_sk: hx(&self.sign.to_bytes()),
            dkg_sk: hx(self.dkg.as_bytes()),
        };
        let r = write_json(path, &f, true);
        f.sign_sk.zeroize();
        f.dkg_sk.zeroize();
        r
    }

    pub fn load(path: &Path) -> WireResult<Self> {
        let mut f: KeyFileJ = read_json(path)?;
        let mut sign_bytes = unhex_n::<32>(&f.sign_sk)?;
        let mut dkg_bytes = unhex_n::<32>(&f.dkg_sk)?;
        f.sign_sk.zeroize();
        f.dkg_sk.zeroize();
        let sign = SigningKey::from_bytes(&sign_bytes);
        let dkg = Option::<Scalar>::from(Scalar::from_canonical_bytes(dkg_bytes));
        sign_bytes.zeroize();
        dkg_bytes.zeroize();
        Ok(NodeIdentity {
            id: f.id,
            url: f.url,
            sign,
            dkg: dkg.ok_or("bad key-generation secret in key file")?,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct ShareFileJ {
    id: u16,
    share: String,
    group: GroupJ,
}

/// Save this node's decryption share and the group key, after key generation.
pub fn save_share(path: &Path, key: &NodeKey, group: &GroupKey) -> WireResult<()> {
    let mut f = ShareFileJ {
        id: key.id(),
        share: hx(key.share_bytes().as_ref()),
        group: GroupJ::from_group(group),
    };
    let r = write_json(path, &f, true);
    f.share.zeroize();
    r
}

pub fn load_share(path: &Path) -> WireResult<(NodeKey, GroupKey)> {
    let mut f: ShareFileJ = read_json(path)?;
    let mut bytes = unhex_n::<32>(&f.share)?;
    f.share.zeroize();
    let key = NodeKey::from_share_bytes(f.id, &bytes).map_err(|e| e.to_string());
    bytes.zeroize();
    let key = key?;
    let group = f.group.to_group()?;
    if group.verifying_share(key.id()) != Some(key.verifying_share()) {
        return Err("share file: the share does not match the group key".into());
    }
    Ok((key, group))
}

/// Where a node keeps the company listings it has accepted.
pub fn listings_path(dir: &Path, id: u16) -> PathBuf {
    dir.join(format!("node-{id}.listings.json"))
}
