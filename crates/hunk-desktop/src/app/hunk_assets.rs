use std::borrow::Cow;

use gpui::{AnyElement, App, AssetSource, IntoElement, RenderOnce, Result, SharedString, Window};
use gpui_component::{Icon, IconNamed, icon_named};
use gpui_component_assets::Assets as ComponentAssets;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
struct EmbeddedAssets;

icon_named!(HunkIconName, "assets/icons", [Debug, Copy, PartialEq, Eq]);

impl From<HunkIconName> for AnyElement {
    fn from(value: HunkIconName) -> Self {
        Icon::new(value).into_any_element()
    }
}

impl RenderOnce for HunkIconName {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}

pub struct HunkAssets;

impl AssetSource for HunkAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if let Some(file) = EmbeddedAssets::get(path) {
            return Ok(Some(file.data));
        }

        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut items: Vec<SharedString> = EmbeddedAssets::iter()
            .filter_map(|asset_path| asset_path.starts_with(path).then(|| asset_path.into()))
            .collect();

        for asset_path in ComponentAssets.list(path)? {
            if !items.iter().any(|existing| existing == &asset_path) {
                items.push(asset_path);
            }
        }

        Ok(items)
    }
}
