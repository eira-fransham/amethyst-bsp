pub use bsp;

use amethyst::{
    assets::{
        Asset, AssetPrefab, Handle, Prefab, PrefabData, ProcessingState, ProgressCounter,
        SimpleFormat,
    },
    derive::PrefabData,
    ecs::{Component, Entity, HashMapStorage, WriteStorage},
    renderer::{MeshData, PosNormTex, Texture, TextureData, TextureMetadata},
    Error,
};
use amethyst_detect_filetype::DetectTextureFormat;
use bsp::Bsp;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const MISSING_TEXTURE_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/missing.png"));

pub struct BspAsset(pub Bsp);

impl Asset for BspAsset {
    type Data = BspAsset;
    type HandleStorage = HashMapStorage<Handle<Self>>;

    const NAME: &'static str = "Bsp";
}

impl From<BspAsset> for Result<ProcessingState<BspAsset>, Error> {
    fn from(other: BspAsset) -> Self {
        Ok(ProcessingState::Loaded(other))
    }
}

#[derive(Clone, Debug)]
pub struct BspFormat;

impl SimpleFormat<BspAsset> for BspFormat {
    type Options = ();

    const NAME: &'static str = "Bsp";

    fn import(&self, bytes: Vec<u8>, _: Self::Options) -> Result<<BspAsset as Asset>::Data, Error> {
        use std::io;

        Bsp::read(io::Cursor::new(bytes))
            .map_err(|e| Error::new(e))
            .map(BspAsset)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PrefabData)]
#[prefab(Component)]
pub struct Cluster {
    pub id: i32,
}

impl Component for Cluster {
    type Storage = HashMapStorage<Self>;
}

#[derive(Default, Deserialize, Serialize, PrefabData)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct BspPrefabElement {
    cluster: Option<Cluster>,
    texture: Option<AssetPrefab<Texture, DetectTextureFormat>>,
    mesh: Option<MeshData>,
}

lazy_static! {
    static ref MISSING_TEXTURE: TextureData = SimpleFormat::import(
        &DetectTextureFormat,
        MISSING_TEXTURE_BYTES.to_vec(),
        TextureMetadata::srgb(),
    )
    .expect("Programmer error: missing texture is invalid");
    static ref MISSING_TEXTURE_FUNCTION: Arc<dyn Fn(amethyst::Error) -> Result<TextureData, amethyst::Error> + Send + Sync + 'static> =
        Arc::new(|_| { Ok(MISSING_TEXTURE.clone()) });
}
impl SimpleFormat<Prefab<BspPrefabElement>> for BspFormat {
    type Options = ();

    const NAME: &'static str = "Bsp";

    fn import(
        &self,
        bytes: Vec<u8>,
        _: Self::Options,
    ) -> Result<<Prefab<BspPrefabElement> as Asset>::Data, Error> {
        use std::io;

        let bsp = Bsp::read(io::Cursor::new(bytes)).map_err(|e| Error::new(e))?;

        let mut prefab = Prefab::new();

        let mut faces = vec![];

        // TODO: We can do this with index buffers instead of vertex buffers
        for (id, cluster) in &bsp.leaves.clusters() {
            let cluster_id = prefab.add(
                Some(0),
                BspPrefabElement {
                    cluster: Some(Cluster { id }),
                    ..Default::default()
                }
                .into(),
            );

            faces.clear();
            faces.extend(
                cluster
                    .into_iter()
                    .flat_map(|leaf| bsp::Handle::new(&bsp, leaf).faces()),
            );
            faces.sort_unstable_by_key(|face| face.texture().map(|t| t.name));

            for (tex, faces) in &faces.iter().group_by(|face| face.texture) {
                let tex = if let Some(texture) = bsp.texture(tex as usize) {
                    texture
                } else {
                    continue;
                };
                if !tex.flags.should_draw() {
                    continue;
                }

                let tex_name = tex.name;

                let verts = faces
                    .flat_map(|face| {
                        face.vertices().map(|vert| PosNormTex {
                            position: [vert.position[0], vert.position[2], -vert.position[1]]
                                .into(),
                            normal: [vert.normal[0], vert.normal[2], -vert.normal[1]].into(),
                            tex_coord: vert.surface_texcoord.into(),
                        })
                    })
                    .collect::<Vec<_>>();

                prefab.add(
                    Some(cluster_id),
                    Some(BspPrefabElement {
                        texture: Some(AssetPrefab::FileOrElse(
                            tex_name.to_string(),
                            DetectTextureFormat,
                            TextureMetadata::srgb(),
                            MISSING_TEXTURE_FUNCTION.clone(),
                        )),
                        mesh: Some(verts.into()),
                        ..Default::default()
                    }),
                );
            }
        }

        for model in bsp.models() {
            faces.clear();
            faces.extend(model.faces());
            faces.sort_unstable_by_key(|face| face.texture().map(|t| t.name));

            for (tex, faces) in &faces.iter().group_by(|face| face.texture) {
                let tex = if let Some(texture) = bsp.texture(tex as usize) {
                    texture
                } else {
                    continue;
                };
                if !tex.flags.should_draw() {
                    continue;
                }

                let tex_name = tex.name;

                let verts = faces
                    .flat_map(|face| {
                        face.vertices().map(|vert| PosNormTex {
                            position: [vert.position[0], vert.position[2], -vert.position[1]]
                                .into(),
                            normal: [vert.normal[0], vert.normal[2], -vert.normal[1]].into(),
                            tex_coord: vert.surface_texcoord.into(),
                        })
                    })
                    .collect::<Vec<_>>();

                prefab.add(
                    None,
                    Some(BspPrefabElement {
                        texture: Some(AssetPrefab::FileOrElse(
                            tex_name.to_string(),
                            DetectTextureFormat,
                            TextureMetadata::srgb(),
                            MISSING_TEXTURE_FUNCTION.clone(),
                        )),
                        mesh: Some(verts.into()),
                        ..Default::default()
                    }),
                );
            }
        }

        Ok(prefab)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
