use std::io::{BufRead, Write};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
struct InsnId(usize);

#[derive(Debug)]
enum Opcode {
    Const(f64),
    VarX,
    VarY,
    VarZ,
    Mul,
    Add,
    Sub,
    Max,
    Min,
    Neg,
    Square,
    Sqrt,
}

impl Opcode {
    fn from_str(name: &str) -> Option<Self> {
        Some(match name {
            "var-x" => Opcode::VarX,
            "var-y" => Opcode::VarY,
            "var-z" => Opcode::VarZ,
            "add" => Opcode::Add,
            "sub" => Opcode::Sub,
            "mul" => Opcode::Mul,
            "neg" => Opcode::Neg,
            "max" => Opcode::Max,
            "min" => Opcode::Min,
            "square" => Opcode::Square,
            "sqrt" => Opcode::Sqrt,
            _ => return None,
        })
    }
}

#[derive(Debug)]
struct Insn {
    opcode: Opcode,
    operands: Vec<InsnId>,
}

#[derive(Debug)]
struct Trace {
    insns: Vec<Insn>,
}

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
                    Insn { opcode: Opcode::Const(float), operands: vec![] }
                }
                Some(word) => {
                    let opcode = Opcode::from_str(word).ok_or(format!("unknown opcode {word}"))?;
                    match opcode {
                        Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Max | Opcode::Min => {
                            let left = vars.get(words.next().ok_or("no left")?).ok_or("unbound variable")?;
                            let right = vars.get(words.next().ok_or("no right")?).ok_or("unbound variable")?;
                            Insn { opcode, operands: vec![*left, *right] }
                        }
                        Opcode::Neg | Opcode::Square | Opcode::Sqrt => {
                            let val = vars.get(words.next().ok_or("no val")?).ok_or("unbound variable")?;
                            Insn { opcode, operands: vec![*val] }
                        }
                        Opcode::VarX | Opcode::VarY | Opcode::VarZ => {
                            Insn { opcode, operands: vec![] }
                        }
                        _ => todo!("{opcode:?}"),
                    }
                }
                None => break,
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
            let value = match insn.opcode {
                Opcode::Const(v) => v,
                Opcode::VarX => x,
                Opcode::VarY => y,
                Opcode::VarZ => z,
                Opcode::Neg => -values[insn.operands[0].0],
                Opcode::Square => values[insn.operands[0].0] * values[insn.operands[0].0],
                Opcode::Sqrt => values[insn.operands[0].0].sqrt(),
                Opcode::Mul => values[insn.operands[0].0] * values[insn.operands[1].0],
                Opcode::Add => values[insn.operands[0].0] + values[insn.operands[1].0],
                Opcode::Sub => values[insn.operands[0].0] - values[insn.operands[1].0],
                Opcode::Max => values[insn.operands[0].0].max(values[insn.operands[1].0]),
                Opcode::Min => values[insn.operands[0].0].min(values[insn.operands[1].0]),
            };
            values[idx] = value;
        }
        *values.last().unwrap()
    }

    fn render_to(&self, filename: &str, height: usize, width: usize)
        -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(filename)?;
        // Write header
        let maxval = 255;
        file.write(format!("P5\n{width} {height}\n{maxval}\n").as_bytes())?;
        for row in 0..height {
            for col in 0..width {
                let x = row as f64 / width as f64;
                let y = col as f64 / height as f64;
                let val = self.eval(x, y, 0.0);
                let brightness = (val.clamp(0.0, 1.0) * (maxval as f64)).round() as u8;
                file.write(&[brightness])?;
            }
        }
        file.flush()?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trace = Trace::from_file("prospero.vm")?;
    trace.render_to("prospero.ppm", /*height=*/600, /*width=*/600)?;
    Ok(())
}
