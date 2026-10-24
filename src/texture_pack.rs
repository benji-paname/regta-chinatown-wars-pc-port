// SPDX-License-Identifier: GPL-3.0-only
//! Local, prevalidated high-resolution assets. PSP logical coordinates stay intact.
use crate::iso::Result;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Clone, Deserialize)]
struct Entry {
    id: usize,
    original_sha256: String,
    original_size: [usize; 2],
    size: [usize; 2],
    file: String,
    source: String,
    rgba_sha256: String,
}
#[derive(Deserialize)]
struct Manifest {
    version: u32,
    entries: Vec<Entry>,
}
#[derive(Clone)]
pub struct Replacement {
    pub size: [usize; 2],
    pub rgba: Vec<[u8; 4]>,
}
struct Pack {
    root: PathBuf,
    entries: HashMap<usize, Entry>,
    cache: Mutex<HashMap<usize, Arc<Replacement>>>,
}
static PACK: OnceLock<Option<Pack>> = OnceLock::new();
impl Pack {
    fn open(root: &Path) -> Result<Self> {
        let manifest: Manifest =
            serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
        if manifest.version != 1 {
            return Err("Unsupported texture pack manifest".into());
        }
        let mut entries = HashMap::new();
        for e in manifest.entries {
            let parts: Vec<_> = Path::new(&e.file).components().collect();
            if parts.len() != 1
                || !matches!(parts[0], Component::Normal(_))
                || e.original_size.contains(&0)
                || e.size.contains(&0)
                || e.size.iter().any(|&x| x > 8192)
                || e.size[0] < e.original_size[0]
                || e.size[1] < e.original_size[1]
                || e.size[0] * e.original_size[1] != e.size[1] * e.original_size[0]
                || e.original_sha256.len() != 64
                || e.rgba_sha256.len() != 64
            {
                return Err("Invalid texture replacement dimensions/path/hash".into());
            }
            if entries.insert(e.id, e).is_some() {
                return Err("Duplicate texture replacement ID".into());
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            entries,
            cache: Mutex::new(HashMap::new()),
        })
    }
    fn get(
        &self,
        id: usize,
        original: &[u8],
        size: [usize; 2],
    ) -> Result<Option<Arc<Replacement>>> {
        let Some(e) = self.entries.get(&id) else {
            return Ok(None);
        };
        // Check before consulting cache: one PSP resource may use different palettes.
        if e.original_size != size || format!("{:x}", Sha256::digest(original)) != e.original_sha256
        {
            return Ok(None);
        }
        let mut cache = self.cache.lock().map_err(|_| "Texture cache lock")?;
        if let Some(t) = cache.get(&id) {
            return Ok(Some(t.clone()));
        }
        let n = e.size[0] * e.size[1] * 4;
        let file = std::fs::File::open(self.root.join(&e.file))?;
        let mut data = Vec::with_capacity(n);
        flate2::read::ZlibDecoder::new(file)
            .take(n as u64 + 1)
            .read_to_end(&mut data)?;
        if data.len() != n || format!("{:x}", Sha256::digest(&data)) != e.rgba_sha256 {
            return Err(format!("Corrupt texture replacement: {}", e.source).into());
        }
        let t = Arc::new(Replacement {
            size: e.size,
            rgba: data
                .chunks_exact(4)
                .map(|p| [p[0], p[1], p[2], p[3]])
                .collect(),
        });
        cache.insert(id, t.clone());
        Ok(Some(t))
    }
}
pub fn initialize(root: Option<&Path>) -> Result<()> {
    let pack = root.map(Pack::open).transpose()?;
    if PACK.set(pack).is_err() {
        return Err("Texture pack already initialized".into());
    }
    Ok(())
}
pub fn replacement(
    id: usize,
    original: &[u8],
    size: [usize; 2],
) -> Result<Option<Arc<Replacement>>> {
    match PACK.get().and_then(|p| p.as_ref()) {
        Some(p) => p.get(id, original, size),
        None => Ok(None),
    }
}
pub fn report() -> serde_json::Value {
    if let Some(p) = PACK.get().and_then(|p| p.as_ref()) {
        let cache = p.cache.lock().unwrap();
        let mut loaded: Vec<_> = cache
            .iter()
            .map(|(id, t)| serde_json::json!({"id":id,"source":p.entries[id].source,"size":t.size}))
            .collect();
        loaded.sort_by_key(|v| v["id"].as_u64());
        serde_json::json!({"enabled":true,"available":p.entries.len(),"loaded":loaded,"logical_psp_dimensions_preserved":true})
    } else {
        serde_json::json!({"enabled":false,"available":0,"loaded":[]})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn replacement_requires_exact_original_and_keeps_alpha() {
        let root = std::env::temp_dir().join(format!("ctw-pack-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let original = b"PSP palette and texels";
        let pixels = [255u8, 1, 2, 0, 3, 4, 5, 128, 6, 7, 8, 255, 9, 10, 11, 255];
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&pixels).unwrap();
        std::fs::write(root.join("1.rgba.z"), z.finish().unwrap()).unwrap();
        let manifest = serde_json::json!({"version":1,"entries":[{"id":1,"original_sha256":format!("{:x}",Sha256::digest(original)),"original_size":[1,1],"size":[2,2],"file":"1.rgba.z","source":"test","rgba_sha256":format!("{:x}",Sha256::digest(pixels))}]});
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        let t = pack.get(1, original, [1, 1]).unwrap().unwrap();
        assert_eq!(t.size, [2, 2]);
        assert_eq!(t.rgba[1], [3, 4, 5, 128]);
        assert!(pack.get(1, b"different palette", [1, 1]).unwrap().is_none());
        assert!(pack.get(2, original, [1, 1]).unwrap().is_none());
        assert!(pack.get(1, original, [2, 2]).unwrap().is_none());
        std::fs::write(root.join("1.rgba.z"), b"broken").unwrap();
        assert!(Pack::open(&root).unwrap().get(1, original, [1, 1]).is_err());
        let mut bad = manifest;
        bad["entries"][0]["file"] = serde_json::json!("../outside");
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&bad).unwrap(),
        )
        .unwrap();
        assert!(Pack::open(&root).is_err());
        std::fs::remove_file(root.join("1.rgba.z")).unwrap();
        std::fs::remove_file(root.join("manifest.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
