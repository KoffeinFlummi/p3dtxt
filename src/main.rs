#[macro_use]
extern crate serde_derive;
extern crate docopt;
extern crate armake2;
extern crate text_io;
extern crate linked_hash_map;

use std::io::{Read, Seek, Write, Cursor, stdin, stdout, BufReader, BufWriter};
use std::fs::{File};
use std::str::{FromStr};
use std::collections::{HashMap};

use docopt::Docopt;
use text_io::*;
use linked_hash_map::LinkedHashMap;

use armake2::io::{Input, Output};
use armake2::p3d::*;

const USAGE: &'static str = "
p3dtxt

Usage:
    p3dtxt bin2txt [-l] [<source> [<target>]]
    p3dtxt txt2bin [<source> [<target>]]
    p3dtxt (-h | --help)
    p3dtxt --version

Commands:
    bin2txt     Convert a regular MLOD P3D into a text-based one.
    txt2bin     Convert a text-based P3D back into a binary one.

Options:
    -l --lossless   Store floats as hex strings to prevent rounding errors.
    -h --help       Show usage information and exit.
       --version    Print the version number and exit.

A note on losslessness:
    Even with the lossless setting, a conversion to text and back might introduce
    some minor differences due to some non-zero padding in files produced by BI
    tools. This padding carries no useful information.
    Consult the README for more information.
";

#[derive(Debug, Deserialize)]
struct Args {
    cmd_bin2txt: bool,
    cmd_txt2bin: bool,
    arg_source: Option<String>,
    arg_target: Option<String>,
    flag_lossless: bool,
    flag_version: bool,
}


fn f32tostr(f: f32, hex: bool) -> String {
    if hex {
        format!("{:x}", f.to_bits())
    } else {
        format!("{}", f)
    }
}

fn strtof32(string: &str, hex: bool) -> f32 {
    if hex {
        f32::from_bits(u32::from_str_radix(string, 16).unwrap())
    } else {
        f32::from_str(string).unwrap()
    }
}

fn hextou32(string: String) -> u32 {
    u32::from_str_radix(&string, 16).unwrap()
}

fn bin2txt<I: Read + Seek, O: Write>(input: &mut I, output: &mut O, lossless: bool) {
    let p3d = P3D::read(input).unwrap();

    let mut writer = BufWriter::new(output);

    writeln!(writer, "v{:x}", p3d.version).unwrap();

    for lod in p3d.lods {
        writeln!(writer, "[{}{}]", if lossless { "0x" } else { "" }, f32tostr(lod.resolution, lossless)).unwrap();
        writeln!(writer, "v{:x}.{:x}", lod.version_major, lod.version_minor).unwrap();

        for point in lod.points {
            writeln!(writer, "P:{},{},{},{:x}",
                f32tostr(point.coords.0, lossless),
                f32tostr(point.coords.1, lossless),
                f32tostr(point.coords.2, lossless),
                point.flags).unwrap();
        }

        for normal in lod.face_normals {
            writeln!(writer, "N:{},{},{}",
                f32tostr(normal.0, lossless),
                f32tostr(normal.1, lossless),
                f32tostr(normal.2, lossless)).unwrap();
        }

        let mut textures: HashMap<String,usize> = HashMap::new();
        let mut materials: HashMap<String,usize> = HashMap::new();

        for face in lod.faces {
            if !textures.contains_key(&face.texture) {
                let i = textures.len();
                textures.insert(face.texture.clone(), i);
                writeln!(writer, "T:{}", face.texture).unwrap();
            }
            if !materials.contains_key(&face.material) {
                let i = materials.len();
                materials.insert(face.material.clone(), i);
                writeln!(writer, "M:{}", face.material).unwrap();
            }

            let verts: Vec<String> = face.vertices.iter().map(|v| format!("{:x},{:x},{},{}",
                v.point_index,
                v.normal_index,
                f32tostr(v.uv.0, lossless),
                f32tostr(v.uv.1, lossless))).collect();

            writeln!(writer, "F:{:x},{:x},{:x},{}",
                textures.get(&face.texture).unwrap(),
                materials.get(&face.material).unwrap(),
                face.flags,
                verts.join(",")).unwrap();
        }

        for (name, buffer) in lod.taggs {
            let hex: Vec<String> = buffer.iter().map(|c| format!("{:02x}", c)).collect();
            writeln!(writer, "TAGG:{}={}", name, hex.join("")).unwrap();
        }
        writeln!(writer, "TAGG:#EndOfFile#=").unwrap();
    }
}

fn txt2bin<I: Read, O: Write>(input: &mut I, output: &mut O) {
    let reader = BufReader::new(input);
    let mut iter = reader.bytes().map(|c| c.unwrap());

    let version: u32 = read!("v{}\n", iter);
    let mut lods: Vec<LOD> = Vec::new();

    loop {
        let res: String = match try_read!("[{}]\n", iter) {
            Ok(res) => res,
            Err(_) => break
        };
        let lossless = res.len() > 2 && &res[..2] == "0x";
        let resolution = strtof32(if lossless { &res[2..] } else { &res }, lossless);

        let version_major: u32 = hextou32(read!("v{}.", iter));
        let version_minor: u32 = hextou32(read!("{}\n", iter));

        let mut points: Vec<Point> = Vec::new();
        let mut normals: Vec<(f32,f32,f32)> = Vec::new();
        let mut textures: Vec<String> = Vec::new();
        let mut materials: Vec<String> = Vec::new();
        let mut faces: Vec<Face> = Vec::new();
        let mut taggs: LinkedHashMap<String, Box<[u8]>> = LinkedHashMap::new();
        loop {
            let (linetype, line): (String, String);
            scan!(iter => "{}:{}\n", linetype, line);

            if linetype == "P" {
                let mut point = Point::new();
                let (x, y, z, pf): (String, String, String, String);
                scan!(line.bytes() => "{},{},{},{}", x, y, z, pf);

                point.coords.0 = strtof32(&x, lossless);
                point.coords.1 = strtof32(&y, lossless);
                point.coords.2 = strtof32(&z, lossless);
                point.flags = hextou32(pf);

                points.push(point);
            } else if linetype == "N" {
                let mut normal: (f32,f32,f32) = (0.0,0.0,0.0);
                let (x, y, z): (String, String, String);
                scan!(line.bytes() => "{},{},{}", x, y, z);

                normal.0 = strtof32(&x, lossless);
                normal.1 = strtof32(&y, lossless);
                normal.2 = strtof32(&z, lossless);

                normals.push(normal);
            } else if linetype == "T" {
                textures.push(line);
            } else if linetype == "M" {
                materials.push(line);
            } else if linetype == "F" {
                let mut face = Face::new();

                let (ti, mi, flags, vertstr): (String, String, String, String);
                scan!(line.bytes() => "{},{},{},{}", ti, mi, flags, vertstr);

                face.texture = textures[hextou32(ti) as usize].clone();
                face.material = materials[hextou32(mi) as usize].clone();
                face.flags = hextou32(flags);

                let segs: Vec<&str> = vertstr.split_terminator(",").collect();
                for i in 0..(segs.len() / 4) {
                    let uv: (f32, f32) = (strtof32(segs[i * 4 + 2], lossless), strtof32(segs[i * 4 + 3], lossless));

                    face.vertices.push(Vertex {
                        point_index: u32::from_str_radix(segs[i * 4 + 0], 16).unwrap(),
                        normal_index: u32::from_str_radix(segs[i * 4 + 1], 16).unwrap(),
                        uv: uv
                    });
                }

                faces.push(face);
            } else if linetype == "TAGG" {
                let segs: Vec<String> = line.split_terminator("=").map(|s| s.to_string()).collect();
                if segs[0] == "#EndOfFile#" { break; }

                let value: Vec<u8> = (0..segs[1].len()).step_by(2)
                    .map(|i| u8::from_str_radix(&segs[1][i..i + 2], 16).unwrap())
                    .collect();
                taggs.insert(segs[0].clone(), value.into_boxed_slice());
            } else {
                unreachable!();
            }
        }

        lods.push(LOD {
            version_major: version_major,
            version_minor: version_minor,
            resolution: resolution,
            points: points,
            face_normals: normals,
            faces: faces,
            taggs: taggs
        });
    }

    let p3d = P3D {
        version: version,
        lods: lods
    };

    p3d.write(output).unwrap();
}

fn get_input(args: &Args) -> Input {
    if let Some(ref s) = args.arg_source {
        Input::File(File::open(s).unwrap())
    } else {
        let mut buffer: Vec<u8> = Vec::new();
        stdin().read_to_end(&mut buffer).unwrap();
        Input::Cursor(Cursor::new(buffer.into_boxed_slice()))
    }
}

fn get_output(args: &Args) -> Output {
    if let Some(ref t) = args.arg_target {
        Output::File(File::create(t).unwrap())
    } else {
        Output::Standard(stdout())
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.deserialize())
                            .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("v0.1.0");
        std::process::exit(0);
    }

    if args.cmd_bin2txt {
        bin2txt(&mut get_input(&args), &mut get_output(&args), args.flag_lossless);
    } else {
        txt2bin(&mut get_input(&args), &mut get_output(&args));
    }
}
