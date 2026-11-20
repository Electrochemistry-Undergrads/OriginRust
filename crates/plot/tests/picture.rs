//! Picture objects on a graph page (T216): drawn into the SVG and into rendered pixels.

use sheaf_core::Project;
use sheaf_core::shape::{Picture, Shape};
use sheaf_core::template::Template;
use std::sync::Arc;

/// A solid red PNG of `w` x `h` pixels.
fn red_png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        let data: Vec<u8> = (0..w * h).flat_map(|_| [255u8, 0, 0, 255]).collect();
        writer.write_image_data(&data).unwrap();
    }
    out
}

#[test]
fn a_picture_object_is_drawn_into_the_svg_and_the_pixels() {
    let mut p = Project::new();
    let g = p.add_graph(Template::Line);
    let picture = Picture { png: Arc::new(red_png(8, 8)), width_px: 8, height_px: 8 };
    p.graph_mut(g).unwrap().layers[0].shapes.push(Shape::picture(picture, (0.25, 0.25), (0.5, 0.5)));

    let svg = sheaf_plot::render_svg(&p, g).unwrap();
    assert!(svg.contains("<image ") && svg.contains("data:image/png;base64,"), "{}", &svg[..svg.len().min(400)]);

    let raster = sheaf_plot::render_raster(&p, g, 1.0).unwrap();
    // The middle of the picture is in the middle of the layer frame.
    let graph = p.graph(g).unwrap();
    let f = graph.layers[0].frame;
    let (pw, ph) = (graph.page.width_pt, graph.page.height_pt);
    let (cx, cy) = ((f.left + f.width * 0.5) * pw, (f.top + f.height * 0.5) * ph);
    let (x, y) = (cx.round() as u32, cy.round() as u32);
    let at = ((y.min(raster.height - 1) * raster.width + x.min(raster.width - 1)) * 4) as usize;
    let px = &raster.rgba[at..at + 3];
    assert!(px[0] > 200 && px[1] < 60 && px[2] < 60, "red picture under the frame centre, got {px:?}");
}
