use anyhow::anyhow;
use gpui::{App, AssetSource, Result, SharedString};
use gpui_kit::assets::Assets as ComponentAssets;
use gpui_kit::component::ThemeRegistry;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
#[include = "icon.png"]
#[include = "icon-light.png"]
#[include = "themes/*.json"]
#[include = "fonts/*.ttf"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        if let Some(f) = ComponentAssets::get(path) {
            return Ok(Some(f.data));
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!(r#"could not find asset at path "{path}""#))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut files: Vec<SharedString> = ComponentAssets::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect();

        files.extend(
            Self::iter()
                .filter_map(|p| p.starts_with(path).then(|| p.into()))
                .collect::<Vec<_>>(),
        );

        Ok(files)
    }
}

/// Register the embedded `assets/themes/*.json` theme sets into the global
/// [`ThemeRegistry`] so they appear in the title-bar theme menu. Adapted from
/// gpui-component's `watch_dir` flow for rust-embedded assets — there is no
/// on-disk themes directory at runtime. Must run after `gpui_kit::component::init`.
pub fn register_themes(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    for path in Assets::iter().filter(|p| p.starts_with("themes/") && p.ends_with(".json")) {
        let Some(file) = Assets::get(path.as_ref()) else {
            continue;
        };
        let Ok(content) = std::str::from_utf8(&file.data) else {
            continue;
        };
        if let Err(err) = registry.load_themes_from_str(content) {
            tracing::warn!(theme = %path, error = %err, "failed to load embedded theme");
        }
    }
}
