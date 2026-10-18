// GENERATED for merkle_root/bn254@1.0.0. Do not edit — `--update` rewrites it.
//
// The loop the harness drives. Three commands, and they divide the work:
//
//     load <case>   read and decode     this file. Not measured.
//     run           call run()          your code. Measured.
//     save <out>    encode and write    this file. Not measured.
//
// Only the call is inside the measured window: no disk, no decoding. And
// nothing of yours runs during `load`, which is why seeing the inputs early is
// not a way to answer early.
mod fherma;
mod solve;

use fherma::{Inputs, Outputs, Point, Tensor};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Instant;

type Fallible = Result<(), Box<dyn std::error::Error>>;

/// Little-endian on the wire, and on every platform this runs on, so the copy
/// is the decode.
trait Wire: Sized + Copy + Default {
    fn from_wire(bytes: &[u8]) -> Self;
    fn to_wire(self) -> Vec<u8>;
}

macro_rules! wire {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn from_wire(bytes: &[u8]) -> Self {
                Self::from_le_bytes(bytes.try_into().expect("width"))
            }
            fn to_wire(self) -> Vec<u8> { self.to_le_bytes().to_vec() }
        }
    )*};
}
wire!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

/// A reader for our own descriptor and nothing else: a number under a key, or
/// a shape array under a name. The schema is fixed and is not going to grow.
fn after(text: &str, key: &str, from: usize) -> Option<usize> {
    let quoted = format!(r#""{key}""#);
    let at = text[from..].find(&quoted)? + from;
    let colon = text[at..].find(':')? + at;
    Some(colon + 1)
}

fn number(text: &str, key: &str) -> Option<f64> {
    let at = after(text, key, 0)?;
    let rest = text[at..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.' && c != 'e')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn shape_of(text: &str, name: &str) -> Option<Vec<i64>> {
    let at = after(text, name, 0)?;
    let open = after(text, "shape", at)?;
    let open = text[open..].find('[')? + open;
    let close = text[open..].find(']')? + open;
    Some(
        text[open + 1..close]
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect(),
    )
}

fn read<T: Wire>(
    dir: &Path,
    name: &str,
    descriptor: &str,
) -> Result<Tensor<T>, Box<dyn std::error::Error>> {
    let shape = shape_of(descriptor, name).ok_or("no shape")?;

    let raw = fs::read(dir.join(format!("{name}.bin")))?;
    let width = std::mem::size_of::<T>();
    let count: usize = shape.iter().product::<i64>() as usize;
    if raw.len() != count * width {
        return Err(format!("{name}: {} bytes for {count} values", raw.len()).into());
    }

    let data = raw.chunks_exact(width).map(T::from_wire).collect();
    Ok(Tensor { shape, data })
}

fn write<T: Wire>(
    dir: &Path,
    name: &str,
    tensor: &Tensor<T>,
    shapes: &mut String,
) -> Fallible {
    let mut raw = Vec::with_capacity(tensor.data.len() * std::mem::size_of::<T>());
    for value in &tensor.data {
        raw.extend_from_slice(&value.to_wire());
    }
    fs::write(dir.join(format!("{name}.bin")), raw)?;

    if !shapes.is_empty() {
        shapes.push(',');
    }
    let dims: Vec<String> = tensor.shape.iter().map(|d| d.to_string()).collect();
    shapes.push_str(&format!(
        r#""{name}":{{"shape":[{}],"elem":"{}"}}"#,
        dims.join(","),
        elem_of::<T>()
    ));
    Ok(())
}

fn elem_of<T>() -> &'static str {
    match std::mem::size_of::<T>() {
        1 => "i8",
        2 => "i16",
        4 => "i32",
        _ => "i64",
    }
}

fn main() -> Fallible {
    let mut state = None;
    let mut inp = Inputs::default();
    let mut answer = Outputs::default();

    for line in io::stdin().lock().lines() {
        let line = line?;
        let (verb, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));

        match verb {
            "init" => {
                let text = fs::read_to_string(rest)?;
                let point_dir = std::path::Path::new(rest).parent().unwrap_or_else(|| std::path::Path::new(".")).join("point");
                let N: u32 = number(&text, "N").unwrap_or(0.0) as u32;
                let L: u32 = number(&text, "L").unwrap_or(0.0) as u32;
                let q = read::<u64>(&point_dir, "q", &format!(r#"{{"q":{{"shape":[{}]}}}}"#, L))?;
                let point = Point { N, L, q };
                let started = Instant::now();
                state = Some(solve::init(&point));
                println!("ready {:.9}", started.elapsed().as_secs_f64());
            }
            "load" => {
                let dir = Path::new(rest);
                let shapes = fs::read_to_string(dir.join("case.json"))?;
                inp.leaves = read(&dir, "leaves", &shapes)?;
                println!("loaded");
            }
            "run" => {
                let held = state.as_mut().ok_or("init was not called")?;
                let started = Instant::now();
                answer = solve::run(held, &inp);
                println!("done {:.9}", started.elapsed().as_secs_f64());
            }
            "save" => {
                let dir = Path::new(rest);
                fs::create_dir_all(dir)?;
                let mut shapes = String::new();
                write(&dir, "root", &answer.root, &mut shapes)?;
                // Built by concatenation rather than a format string: the
                // braces would have to survive three layers of escaping to get
                // here, and one of them already lost.
                let body = String::from(r#"{"shapes":{"#) + &shapes + "}}";
                fs::write(dir.join("case.json"), body)?;
                println!("saved");
            }
            "stop" => {
                if let Some(held) = state.take() {
                    solve::free(held);
                }
                println!("bye");
                io::stdout().flush()?;
                return Ok(());
            }
            _ => {}
        }
        io::stdout().flush()?;
    }
    Ok(())
}
