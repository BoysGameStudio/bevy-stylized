use super::state::{DebugMode, OrbitCamera, Recipe};
use bevy::prelude::*;
use bevy_stylized::{StylizedMaterialSet, prelude::*};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

pub struct Input {
    pub root: PathBuf,
    pub name: String,
    pub set: StylizedMaterialSet,
    pub view: StylizedViewDescriptor,
    pub files: BTreeMap<String, String>,
    pub meshes: usize,
    pub focus: Option<[f32; 3]>,
}
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}
pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn local(root: &Path, uri: &str) -> Result<PathBuf, String> {
    if uri.is_empty()
        || uri.contains([':', '\\'])
        || Path::new(uri)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("unsupported bundle path: {uri}"));
    }
    let p = root
        .join(uri)
        .canonicalize()
        .map_err(|e| format!("{uri}: {e}"))?;
    if !p.starts_with(root) {
        return Err("bundle symlink escapes asset root".into());
    }
    Ok(p)
}
impl Input {
    pub fn load(path: &Path) -> Result<Self, String> {
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if path.extension().and_then(|s| s.to_str()) != Some("glb") {
            return Err("a compiled .glb is required".into());
        }
        let root = path.parent().ok_or("asset directory")?.to_path_buf();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("UTF-8 asset name required")?
            .to_owned();
        let side = path.with_extension("npr.json");
        let view_path = path.with_extension("view.json");
        local(
            &root,
            side.file_name()
                .unwrap()
                .to_str()
                .ok_or("UTF-8 sidecar name required")?,
        )?;
        local(
            &root,
            view_path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or("UTF-8 view name required")?,
        )?;
        let sidecar =
            bevy_stylized::stylized_gltf::load_sidecar(&side).map_err(|e| e.to_string())?;
        if sidecar.materials.is_empty() {
            return Err("compiled material descriptors are required".into());
        }
        let view: StylizedViewDescriptor =
            serde_json::from_slice(&read(&view_path)?).map_err(|e| e.to_string())?;
        let bytes = read(&path)?;
        let glb = gltf::Gltf::from_slice(&bytes).map_err(|e| e.to_string())?;
        fn count(node: gltf::Node<'_>) -> usize {
            node.mesh().map_or(0, |m| m.primitives().count())
                + node.children().map(count).sum::<usize>()
        }
        let meshes = glb
            .scenes()
            .next()
            .ok_or("GLB has no scene")?
            .nodes()
            .map(count)
            .sum();
        if meshes == 0 {
            return Err("GLB scene 0 has no renderable primitives".into());
        }
        let mut files = BTreeMap::new();
        for p in [&path, &side, &view_path] {
            files.insert(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                sha(&read(p)?),
            );
        }
        let mut uris = sidecar
            .materials
            .values()
            .flat_map(|m| m.textures.iter().map(|t| t.uri.clone()))
            .collect::<Vec<_>>();
        for b in glb.buffers() {
            if let gltf::buffer::Source::Uri(u) = b.source() {
                uris.push(u.into());
            }
        }
        for i in glb.images() {
            if let gltf::image::Source::Uri { uri, .. } = i.source() {
                uris.push(uri.into());
            }
        }
        for uri in uris {
            files.insert(uri.clone(), sha(&read(&local(&root, &uri)?)?));
        }
        // Optional explicit focus avoids depending on a mesh named "Face".
        let focus_path = root.join("inspector-focus.json");
        let focus = if focus_path.exists() {
            let b = read(&local(&root, "inspector-focus.json")?)?;
            files.insert("inspector-focus.json".into(), sha(&b));
            let p: [f32; 3] = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
            if !Vec3::from_array(p).is_finite() {
                return Err("invalid explicit focus".into());
            }
            Some(p)
        } else {
            None
        };
        Ok(Self {
            root,
            name,
            set: StylizedMaterialSet {
                materials: sidecar.materials,
                texture_prefix: String::new(),
            },
            view,
            files,
            meshes,
            focus,
        })
    }
    pub fn supports_debug(&self) -> bool {
        self.set
            .materials
            .values()
            .all(|m| m.surface_class == StylizedSurfaceClass::Hero)
    }
    pub fn recipe(&self, resolution: u32) -> Recipe {
        let camera = OrbitCamera {
            target: [0., 1., 0.],
            yaw: 0.,
            pitch: 0.,
            distance: 3.,
        };
        Recipe {
            schema: "bevy_stylized.inspector/v1".into(),
            files: self.files.clone(),
            cameras: [camera.clone(), camera],
            linked: true,
            debug: [
                DebugMode::Beauty,
                if self.supports_debug() {
                    DebugMode::Lighting
                } else {
                    DebugMode::Beauty
                },
            ],
            light: self.view.key_light_direction.normalize().to_array(),
            exposure: self.view.exposure,
            resolution,
        }
    }
    pub fn recipe_from(&self, path: &Path) -> Result<Recipe, String> {
        let r: Recipe = serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())?;
        r.validate()?;
        if r.files != self.files {
            return Err("recipe asset fingerprint does not match this compiled bundle".into());
        }
        Ok(r)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_paths_cannot_escape_or_be_urls() {
        let temp = tempfile::tempdir().unwrap();
        for p in ["../asset.glb", "/asset.glb", "https://x/a", "x\\a"] {
            assert!(local(temp.path(), p).is_err());
        }
    }
    #[test]
    fn saved_recipe_requires_exact_asset_identity() {
        let temp = tempfile::tempdir().unwrap();
        let input = Input {
            root: temp.path().to_path_buf(),
            name: "character.glb".into(),
            set: StylizedMaterialSet {
                materials: Default::default(),
                texture_prefix: String::new(),
            },
            view: StylizedViewDescriptor::default(),
            files: BTreeMap::from([("character.glb".into(), sha(b"original"))]),
            meshes: 1,
            focus: None,
        };
        let mut recipe = input.recipe(512);
        let path = temp.path().join("recipe.json");
        std::fs::write(&path, serde_json::to_vec(&recipe).unwrap()).unwrap();
        assert!(input.recipe_from(&path).is_ok());
        recipe.files.insert("character.glb".into(), sha(b"changed"));
        std::fs::write(&path, serde_json::to_vec(&recipe).unwrap()).unwrap();
        assert!(
            input
                .recipe_from(&path)
                .unwrap_err()
                .contains("fingerprint")
        );
    }
}
