use std::io::{BufRead, Write};
use std::collections::HashMap;
use dynasmrt::dynasm;
use dynasmrt::DynasmApi;

#[derive(Clone, Copy, Debug)]
struct InsnId(usize);

#[derive(Debug, Copy, Clone)]
enum Insn {
    Const(f64),
    VarX,
    VarY,
    VarZ,
    Mul(InsnId, InsnId),
    Add(InsnId, InsnId),
    Sub(InsnId, InsnId),
    Max(InsnId, InsnId),
    Min(InsnId, InsnId),
    Neg(InsnId),
    Square(InsnId),
    Sqrt(InsnId),
}

#[derive(Debug)]
struct Trace {
    insns: Vec<Insn>,
}

type Compiled = extern "C" fn(values: *mut f64, x: f64, y: f64, z: f64) -> f64;

impl Trace {
    fn new() -> Self {
        Self { insns: Default::default() }
    }

    fn push_insn(&mut self, insn: Insn) -> InsnId {
        self.insns.push(insn);
        InsnId(self.insns.len()-1)
    }

    fn from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(filename)?;
        let mut result = Self::new();
        let mut vars: HashMap<String, InsnId> = HashMap::new();
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if line.starts_with('#') { continue; }
            let mut words = line.split(' ');
            let dst = match words.next() {
                Some(name) => name,
                None => break,
            };
            let insn = match words.next() {
                Some("const") => {
                    let float = words.next().ok_or("no float")?.parse::<f64>()?;
                    Insn::Const(float)
                }
                Some("var-x") => Insn::VarX,
                Some("var-y") => Insn::VarY,
                Some("var-z") => Insn::VarZ,
                Some("add") => {
                    let left = *vars.get(words.next().ok_or("no left")?).ok_or("unbound variable")?;
                    let right = *vars.get(words.next().ok_or("no right")?).ok_or("unbound variable")?;
                    Insn::Add(left, right)
                }
                Some("sub") => {
                    let left = *vars.get(words.next().ok_or("no left")?).ok_or("unbound variable")?;
                    let right = *vars.get(words.next().ok_or("no right")?).ok_or("unbound variable")?;
                    Insn::Sub(left, right)
                }
                Some("mul") => {
                    let left = *vars.get(words.next().ok_or("no left")?).ok_or("unbound variable")?;
                    let right = *vars.get(words.next().ok_or("no right")?).ok_or("unbound variable")?;
                    Insn::Mul(left, right)
                }
                Some("min") => {
                    let left = *vars.get(words.next().ok_or("no left")?).ok_or("unbound variable")?;
                    let right = *vars.get(words.next().ok_or("no right")?).ok_or("unbound variable")?;
                    Insn::Min(left, right)
                }
                Some("max") => {
                    let left = *vars.get(words.next().ok_or("no left")?).ok_or("unbound variable")?;
                    let right = *vars.get(words.next().ok_or("no right")?).ok_or("unbound variable")?;
                    Insn::Max(left, right)
                }
                Some("neg") => {
                    let val = *vars.get(words.next().ok_or("no val")?).ok_or("unbound variable")?;
                    Insn::Neg(val)
                }
                Some("square") => {
                    let val = *vars.get(words.next().ok_or("no val")?).ok_or("unbound variable")?;
                    Insn::Square(val)
                }
                Some("sqrt") => {
                    let val = *vars.get(words.next().ok_or("no val")?).ok_or("unbound variable")?;
                    Insn::Sqrt(val)
                }
                word => todo!("{word:?}"),
            };
            let insn_id = result.push_insn(insn);
            vars.insert(dst.into(), insn_id);
        }
        Ok(result)
    }

    fn eval(&self, x: f64, y: f64, z: f64) -> f64 {
        assert!(!self.insns.is_empty(), "Must have some value to return");
        let mut values = vec![0.0; self.insns.len()];
        for (idx, insn) in self.insns.iter().enumerate() {
            let value = match *insn {
                Insn::Const(v) => v,
                Insn::VarX => x,
                Insn::VarY => y,
                Insn::VarZ => z,
                Insn::Neg(val) => -values[val.0],
                Insn::Square(val) => values[val.0] * values[val.0],
                Insn::Sqrt(val) => values[val.0].sqrt(),
                Insn::Mul(left, right) => values[left.0] * values[right.0],
                Insn::Add(left, right) => values[left.0] + values[right.0],
                Insn::Sub(left, right) => values[left.0] - values[right.0],
                Insn::Max(left, right) => values[left.0].max(values[right.0]),
                Insn::Min(left, right) => values[left.0].min(values[right.0]),
            };
            values[idx] = value;
        }
        *values.last().unwrap()
    }

    fn compile(&self) -> (dynasmrt::ExecutableBuffer, Compiled) {
        assert!(!self.insns.is_empty(), "Must have some value to return");
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        let begin = ops.offset();
        for (idx, insn) in self.insns.iter().enumerate() {
            match *insn {
                Insn::Const(v) =>
                    dynasm!(ops
                        ; movabs rax, v.to_bits() as i64
                        ; mov QWORD [rdi + (idx as i32) * 8], rax
                    ),
                Insn::VarX =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::VarY =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd [rdi + (idx as i32) * 8], xmm1
                    ),
                Insn::VarZ =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd [rdi + (idx as i32) * 8], xmm2
                    ),
                Insn::Mul(left, right) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (left.0 as i32) * 8]
                        ; mulsd xmm0, [rdi + (right.0 as i32) * 8]
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::Add(left, right) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (left.0 as i32) * 8]
                        ; addsd xmm0, [rdi + (right.0 as i32) * 8]
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::Sub(left, right) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (left.0 as i32) * 8]
                        ; subsd xmm0, [rdi + (right.0 as i32) * 8]
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::Max(left, right) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (left.0 as i32) * 8]
                        ; maxsd xmm0, [rdi + (right.0 as i32) * 8]
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::Min(left, right) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (left.0 as i32) * 8]
                        ; minsd xmm0, [rdi + (right.0 as i32) * 8]
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::Neg(val) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movabs rax, -9223372036854775808
                        ; xor rax, [rdi + (val.0 as i32) * 8]
                        ; mov QWORD [rdi + (idx as i32) * 8], rax
                    ),
                Insn::Square(val) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (val.0 as i32) * 8]
                        ; mulsd xmm0, xmm0
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
                Insn::Sqrt(val) =>
                    dynasm!(ops
                        ; .arch x64
                        ; movsd xmm0, [rdi + (val.0 as i32) * 8]
                        ; sqrtsd xmm0, xmm0
                        ; movsd [rdi + (idx as i32) * 8], xmm0
                    ),
            }
        }
        dynasm!(ops
            ; .arch x64
            ; movsd xmm0, [rdi + ((self.insns.len()-1) as i32) * 8]
            ; ret
        );
        let buf = ops.finalize().unwrap();
        let compiled: Compiled = unsafe { std::mem::transmute(buf.ptr(begin)) };
        (buf, compiled)
    }

    fn render_to(&self, filename: &str, height: usize, width: usize)
        -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(filename)?;
        // Write header
        let maxval = 255;
        file.write(format!("P5\n{width} {height}\n{maxval}\n").as_bytes())?;
        let mut data = vec![0; width*height];
        let fwidth = width as f64;
        let fheight = height as f64;
        let minx = -1.0;
        let maxx = 1.0;
        let miny = -1.0;
        let maxy = 1.0;
        for row in 0..height {
            let frow = row as f64;
            for col in 0..width {
                let fcol = col as f64;
                let x: f64 = minx + (maxx - minx) * fcol / fwidth;
                let y: f64 = miny + (maxy - miny) * frow / fheight;
                let val = self.eval(x, -y, 0.0);
                data[row*width + col] = if val < 0.0 { maxval } else { 0 };
            }
        }
        file.write_all(&data)?;
        file.flush()?;
        Ok(())
    }

    fn compiled_render_to(&self, filename: &str, height: usize, width: usize, f: Compiled)
        -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(filename)?;
        // Write header
        let maxval = 255;
        file.write(format!("P5\n{width} {height}\n{maxval}\n").as_bytes())?;
        let mut data = vec![0; width*height];
        let fwidth = width as f64;
        let fheight = height as f64;
        let minx = -1.0;
        let maxx = 1.0;
        let miny = -1.0;
        let maxy = 1.0;
        for row in 0..height {
            let frow = row as f64;
            for col in 0..width {
                let fcol = col as f64;
                let x: f64 = minx + (maxx - minx) * fcol / fwidth;
                let y: f64 = miny + (maxy - miny) * frow / fheight;
                let mut values = vec![0.0; self.insns.len()];
                let val = f(values.as_mut_ptr(), x, -y, 0.0);
                data[row*width + col] = if val < 0.0 { maxval } else { 0 };
            }
        }
        file.write_all(&data)?;
        file.flush()?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trace = Trace::from_file("prospero.vm")?;
    let (_buf, compiled) = trace.compile();
    // let result = compiled(values.as_mut_ptr(), 0.0, 0.0, 0.0);
    // eprintln!("result: {result}");
    // trace.compiled_render_to("prospero.ppm", /*height=*/200, /*width=*/200, compiled)?;
    trace.render_to("prospero.ppm", /*height=*/200, /*width=*/200)?;
    Ok(())
}
