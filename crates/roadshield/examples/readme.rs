//! The README example: renders New Jersey county route 609 to `cr609.svg`.
//! Run from the repository root: `cargo run -p roadshield --example readme`.

use std::collections::HashMap;
use std::path::Path;

use roadshield::{DisplayContext, Engine, Rendering, ResourcePack, RouteDescriptor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Every file of the pack, keyed by its path relative to the pack root.
    let root = Path::new("packs/americana");
    let mut files = HashMap::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let key = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(key, std::fs::read(&path)?);
            }
        }
    }

    let engine = Engine::new(ResourcePack::load(&files)?)?;
    let route = RouteDescriptor::new("US:NJ:CR", "609");
    if let Rendering::Symbol(shield) = engine.render(&route, &DisplayContext::default())? {
        std::fs::write("cr609.svg", &shield.svg)?;
    }
    Ok(())
}
