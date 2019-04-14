pub use bsp;

use amethyst::{
    assets::{
        self, Asset, AssetPrefab, Handle, Prefab, PrefabData, PrefabError, ProcessingState,
        ProgressCounter, SimpleFormat,
    },
    derive::PrefabData,
    ecs::{Component, Entity, HashMapStorage, WriteStorage},
    renderer::{MeshData, PosNormTex, Texture, TextureFormat, TextureMetadata},
};
use bsp::Bsp;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub struct BspAsset(pub Bsp);

impl Asset for BspAsset {
    type Data = BspAsset;
    type HandleStorage = HashMapStorage<Handle<Self>>;

    const NAME: &'static str = "Bsp";
}

impl From<BspAsset> for Result<ProcessingState<BspAsset>, assets::Error> {
    fn from(other: BspAsset) -> Self {
        Ok(ProcessingState::Loaded(other))
    }
}

#[derive(Clone, Debug)]
pub struct BspFormat;

impl SimpleFormat<BspAsset> for BspFormat {
    type Options = ();

    const NAME: &'static str = "Bsp";

    fn import(
        &self,
        bytes: Vec<u8>,
        _: Self::Options,
    ) -> Result<<BspAsset as Asset>::Data, assets::Error> {
        use std::io;

        Bsp::read(io::Cursor::new(bytes))
            .map_err(|e| {
                assets::Error::with_chain(
                    e,
                    assets::ErrorKind::Msg(format!("Failed to import BSP")),
                )
            })
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
    texture: Option<AssetPrefab<Texture, TextureFormat>>,
    mesh: Option<MeshData>,
}

impl SimpleFormat<Prefab<BspPrefabElement>> for BspFormat {
    type Options = ();

    const NAME: &'static str = "Bsp";

    fn import(
        &self,
        bytes: Vec<u8>,
        _: Self::Options,
    ) -> Result<<Prefab<BspPrefabElement> as Asset>::Data, assets::Error> {
        use std::io;

        let bsp = Bsp::read(io::Cursor::new(bytes)).map_err(|e| {
            assets::Error::with_chain(e, assets::ErrorKind::Msg(format!("Failed to import BSP")))
        })?;

        let mut prefab = Prefab::new();

        let mut faces = vec![];

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

            for (tex_name, faces) in &faces.iter().group_by(|face| face.texture().map(|t| t.name)) {
                let verts = faces
                    .flat_map(|face| {
                        face.vertices().map(|vert| PosNormTex {
                            position: vert.position.into(),
                            normal: vert.normal.into(),
                            tex_coord: vert.surface_texcoord.into(),
                        })
                    })
                    .collect::<Vec<_>>();

                prefab.add(
                    Some(cluster_id),
                    Some(BspPrefabElement {
                        texture: tex_name.map(|tex_name| {
                            AssetPrefab::File(
                                format!("{}.png", tex_name),
                                TextureFormat::Png,
                                TextureMetadata::srgb(),
                            )
                        }),
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
