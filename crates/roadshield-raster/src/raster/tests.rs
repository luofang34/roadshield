use roadshield::{DisplayContext, Engine, ResourcePack, RouteDescriptor, ShieldSymbol};

use super::*;

fn symbol(network: &str, r: &str) -> ShieldSymbol {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/americana");
    let mut files = std::collections::HashMap::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                files.insert(
                    p.strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read(&p).unwrap(),
                );
            }
        }
    }
    let engine = Engine::new(ResourcePack::load(&files).unwrap()).unwrap();
    *engine
        .render(
            &RouteDescriptor::new(network, r),
            &DisplayContext::default(),
        )
        .unwrap()
        .symbol()
        .cloned()
        .map(Box::new)
        .unwrap()
}

#[test]
fn rasterizes_at_device_pixel_ratio() {
    let s = symbol("US:I", "287");
    let img = rasterize(
        &s,
        &RasterOptions {
            pixel_ratio: 2.0,
            ..RasterOptions::default()
        },
    )
    .unwrap();
    assert_eq!((img.width, img.height), (50, 40));
    assert_eq!(img.data.len(), 50 * 40 * 4);
    assert_eq!(img.pixel_ratio_milli, 2000);
    // Centre of the blue lower half of the Interstate blank is opaque blue-ish.
    let px = |x: u32, y: u32| &img.data[((y * img.width + x) * 4) as usize..][..4];
    assert_eq!(px(25, 36)[3], 255);
    assert!(px(25, 36)[2] > px(25, 36)[0]);
    // Corners outside the shield are transparent.
    assert_eq!(px(0, 39)[3], 0);
}

#[test]
fn raster_keys_separate_ratio_and_alpha() {
    let s = symbol("US:US", "22");
    let a = raster_key(&s.semantic_key, &RasterOptions::default());
    let b = raster_key(
        &s.semantic_key,
        &RasterOptions {
            pixel_ratio: 2.0,
            ..RasterOptions::default()
        },
    );
    let c = raster_key(
        &s.semantic_key,
        &RasterOptions {
            alpha: AlphaMode::Straight,
            ..RasterOptions::default()
        },
    );
    assert!(a != b && a != c && b != c);
    assert!(a.starts_with(&s.semantic_key));
}

#[test]
fn straight_and_premultiplied_agree_on_opaque_pixels() {
    let s = symbol("US:NJ:CR", "609");
    let pm = rasterize(&s, &RasterOptions::default()).unwrap();
    let st = rasterize(
        &s,
        &RasterOptions {
            alpha: AlphaMode::Straight,
            ..RasterOptions::default()
        },
    )
    .unwrap();
    for (a, b) in pm.data.chunks(4).zip(st.data.chunks(4)) {
        if a[3] == 255 {
            assert_eq!(a, b);
        }
        assert_eq!(a[3], b[3]);
    }
}

#[test]
fn limits_and_png_encoding() {
    let s = symbol("US:I", "287");
    assert!(matches!(
        rasterize(
            &s,
            &RasterOptions {
                pixel_ratio: 0.0,
                ..RasterOptions::default()
            }
        ),
        Err(RasterError::Options(_))
    ));
    assert!(matches!(
        rasterize(
            &s,
            &RasterOptions {
                pixel_ratio: 8.0,
                max_pixels: 100,
                ..RasterOptions::default()
            }
        ),
        Err(RasterError::TooLarge { .. })
    ));
    let png = encode_png(&rasterize(&s, &RasterOptions::default()).unwrap()).unwrap();
    assert_eq!(&png[1..4], b"PNG");
}
