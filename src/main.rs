use std::fs::File;
use std::io::Read;

use clap::{ArgGroup, Parser};
use obj_exporter::{export_to_file, Geometry, Object, ObjSet, Primitive, Shape, VertexIndex};

#[derive(Debug, Copy, Clone)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Copy, Clone)]
struct Face {
    a: u32,
    b: u32,
    c: u32,
}

#[derive(Debug)]
struct Vertices {
    header: u32,
    len: u32,
    vertices: Vec<Vertex>,
    footer: u32,
}

impl Vertices {
    pub fn new() -> Vertices {
        Vertices {
            header: 0,
            len: 0,
            vertices: Vec::new(),
            footer: 0,
        }
    }
}

#[derive(Debug)]
struct Faces {
    header: u32,
    len: u32,
    faces: Vec<Face>,
    footer: u32,
}

impl Faces {
    pub fn new() -> Faces {
        Faces {
            header: 0,
            len: 0,
            faces: vec![],
            footer: 0,
        }
    }
}

#[derive(Debug)]
struct Normals {
    header: u32,
    len: u32,
    normals: Vec<Vertex>,
    footer: u32,
}

impl Normals {
    pub fn new() -> Normals {
        Normals {
            header: 0,
            len: 0,
            normals: vec![],
            footer: 0,
        }
    }
}

#[derive(Debug)]
struct Group {
    header: u32,
    faces: Faces,
    normals: Normals,
    footer: u32,
}

impl Group {
    pub fn new() -> Group {
        Group {
            header: 0,
            faces: Faces::new(),
            normals: Normals::new(),
            footer: 0,
        }
    }
}

#[derive(Debug)]
struct BMF {
    header: u32,
    vertices: Vertices,
    group: Group,
    footer: u32,
}

impl BMF {
    pub fn new() -> BMF {
        BMF {
            header: 0,
            vertices: Vertices::new(),
            group: Group::new(),
            footer: 0,
        }
    }
}

fn as_vertex_le(array: &[u8; 12]) -> Vertex {
    Vertex {
        x: f32::from_le_bytes(<[u8; 4]>::try_from(&array[0..4]).unwrap()),
        y: f32::from_le_bytes(<[u8; 4]>::try_from(&array[4..8]).unwrap()),
        z: f32::from_le_bytes(<[u8; 4]>::try_from(&array[8..12]).unwrap()),
    }
}

fn as_face_le(array: &[u8; 12]) -> Face {
    Face {
        a: u32::from_le_bytes(<[u8; 4]>::try_from(&array[0..4]).unwrap()),
        b: u32::from_le_bytes(<[u8; 4]>::try_from(&array[4..8]).unwrap()),
        c: u32::from_le_bytes(<[u8; 4]>::try_from(&array[8..12]).unwrap()),
    }
}

/// Naive parsing of BMF format, which roughly is:
///
/// ```
/// BMF Header
///     Vertices Header u32
///         Vertices Len u32
///         Vertices Data (Len * 3 * f32)
///     Vertices Footer u32
///
///     Group Header u32
///         Faces Header u32
///             Faces Len u32
///             Faces Data (Len * 3 * u32)
///         Faces Footer u32
///
///         Normals Header u32
///             Normals Len u32
///             Normals Data (Len * 3 * f32)
///         Normals Footer u32
///     Group Footer u32
/// BMF Footer u32
/// ```
///
/// There's potentially more advanced variations of these files, but the
/// ones discovered so far only have this rigid format, as documented
/// on http://paulbourke.net/dataformats/bmf_2/
///
/// # Arguments
///
/// * `reader`: reader implementing std::io::Read
///
/// returns: BMF
///
fn load_bmf<R>(reader: &mut R) -> Result<BMF, std::io::Error> where R: Read {
    let mut bmf: BMF = BMF::new();
    let mut buffer = [0; 4];

    reader.read_exact(&mut buffer)?;
    bmf.header = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.vertices.header = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.vertices.len = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    for _i in 0..bmf.vertices.len {
        let mut vert_buf = [0; 12];

        reader.read_exact(&mut vert_buf)?;
        let vert = as_vertex_le(&vert_buf);
        bmf.vertices.vertices.push(vert);
    }

    reader.read_exact(&mut buffer)?;
    bmf.vertices.footer = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.group.header = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.group.faces.header = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.group.faces.len = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());


    for _i in 0..bmf.group.faces.len {
        let mut face_buf = [0; 12];

        reader.read_exact(&mut face_buf)?;
        let face = as_face_le(&face_buf);
        bmf.group.faces.faces.push(face);
    }

    reader.read_exact(&mut buffer)?;
    bmf.group.faces.footer = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.group.normals.header = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.group.normals.len = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    for _i in 0..bmf.group.normals.len {
        let mut norm_buf = [0; 12];

        reader.read_exact(&mut norm_buf)?;
        let norm = as_vertex_le(&norm_buf);
        bmf.group.normals.normals.push(norm);
    }

    reader.read_exact(&mut buffer)?;
    bmf.group.normals.footer = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.group.footer = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    reader.read_exact(&mut buffer)?;
    bmf.footer = u32::from_le_bytes(<[u8; 4]>::try_from(buffer).unwrap());

    return Ok(bmf);
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(group(ArgGroup::new("source").required(true).args(["file", "url"])))]
struct Cli {
    #[arg(long)]
    file: Option<String>,
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    out: String,
}

fn main() {
    let cli: Cli = Cli::parse();

    // match method to pull source data to create bmf
    let bmf: BMF = match (cli.file, cli.url) {
        (None, Some(url)) => {
            let mut body = reqwest::blocking::get(url).expect("Failed to open url");
            load_bmf(&mut body).expect("Failed to load BMF")
        }
        (Some(file), None) => {
            let mut file = File::open(file).expect("Failed to open file");
            load_bmf(&mut file).expect("Failed to load BMF")
        }
        (_, _) => unreachable!()
    };

    // convert from BMF vertices to OBJ vertices (just a cast from u32 to u64)
    let obj_verts: Vec<obj_exporter::Vertex> = bmf.vertices.vertices.iter().
        map(|v| obj_exporter::Vertex {
            x: v.x as f64,
            y: v.y as f64,
            z: v.z as f64,
        }).collect();

    // build up geometry for OBJ from BMF faces, don't include normals or texture
    // since we don't have texture and normals need to be recomputed anyways
    let obj_shapes: Vec<Shape> = bmf.group.faces.faces.iter().
        map(|&f| Shape {
            primitive: Primitive::Triangle(
                (f.a as VertexIndex, None, None),
                (f.b as VertexIndex, None, None),
                (f.c as VertexIndex, None, None),
            ),
            groups: vec![],
            smoothing_groups: vec![],
        }).collect();

    // build the object and object set (only one of each) to export
    let object_set = ObjSet {
        material_library: None,
        objects: vec![Object {
            name: "".to_string(),
            vertices: obj_verts,
            tex_vertices: vec![],
            normals: vec![],
            geometry: vec![Geometry {
                material_name: None,
                shapes: obj_shapes,
            }],
        }],
    };

    export_to_file(&object_set, cli.out).expect("failed to export obj");
}