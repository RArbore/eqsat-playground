use std::env::args;
use std::fs::read_to_string;
use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;

use util::arena::Arena;
use util::interner::StringInterner;
use util::union_find::ClassId;

use imp::ai::abstract_interpret;
use imp::grammar::ProgramParser;
use imp::ssa::{Graph, Term};

pub fn main() {
    let mut string_buf: [u8; 100] = [0; 100];
    let string_arena = Arena::new_backed(&mut string_buf);
    let mut interner = StringInterner::new(&string_arena);
    let mut buf: [u8; 10000] = [0; 10000];
    let arena = Arena::new_backed(&mut buf);

    let path = args().skip(1).next().unwrap();
    let program = read_to_string(path).unwrap();
    let program = ProgramParser::new()
        .parse(&arena, &mut interner, &program)
        .unwrap();
    let graphs = abstract_interpret(&program, &mut interner);
    for mut graph in graphs {
        graph.rebuild();
        let dot = dot(&graph);

        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", dot).unwrap();
        Command::new("xdot").arg(tmp.path()).status().unwrap();
    }
}

pub fn dot(graph: &Graph) -> String {
    let mut s = "digraph EGraph {\ncompound=true\n".to_string();
    let open = |s: &mut String, id: ClassId| {
        *s = format!(
            "{}subgraph cluster_{} {{\nnode_{} [shape=point style=invis]\n",
            s,
            id.idx(),
            id.idx()
        );
    };
    let node = |s: &mut String, name: &str, label: &str| {
        *s = format!("{}{} [label=\"{}\"]\n", s, name, label);
    };
    let link = |s: &mut String, src: &str, dst: ClassId| {
        *s = format!(
            "{}node_{} -> {} [ltail=\"cluster_{}\"]\n",
            s,
            dst.idx(),
            src,
            dst.idx()
        );
    };
    let close = |s: &mut String| {
        *s = format!("{}}}\n", s);
    };
    for term in graph.terms() {
        use Term::*;
        let name = match term {
            Constant { value, .. } => format!("cons_{}", value as u32),
            Param { index, .. } => format!("param_{}", index),
            Start { .. } => "start".to_string(),
            Region { lhs, rhs, .. } => format!("region_{}_{}", lhs.idx(), rhs.idx()),
            Branch { pred, cond, .. } => format!("branch_{}_{}", pred.idx(), cond.idx()),
            ControlProj { pred, index, .. } => format!("control_proj_{}_{}", pred.idx(), index),
            Finish { pred, value, .. } => format!("finish_{}_{}", pred.idx(), value.idx()),
            Phi {
                region, lhs, rhs, ..
            } => format!("phi_{}_{}_{}", region.idx(), lhs.idx(), rhs.idx()),
            Add { lhs, rhs, .. } => format!("add_{}_{}", lhs.idx(), rhs.idx()),
        };
        let label = match term {
            Constant { value, .. } => format!("{}", value),
            Param { index, .. } => format!("Param #{}", index),
            Start { .. } => "Start".to_string(),
            Region { .. } => format!("Region"),
            Branch { .. } => format!("Branch"),
            ControlProj { index, .. } => format!("π({})", index),
            Finish { .. } => format!("Finish"),
            Phi { .. } => format!("ϕ"),
            Add { .. } => format!("+"),
        };
        let root = term.root();
        open(&mut s, root);
        node(&mut s, &name, &label);
        close(&mut s);
        match term {
            Constant { .. } | Param { .. } | Start { .. } => {}
            Branch { pred, cond, .. } => {
                link(&mut s, &name, pred);
                link(&mut s, &name, cond);
            }
            ControlProj { pred, .. } => {
                link(&mut s, &name, pred);
            }
            Finish { pred, value, .. } => {
                link(&mut s, &name, pred);
                link(&mut s, &name, value);
            }
            Region { lhs, rhs, .. } | Add { lhs, rhs, .. } => {
                link(&mut s, &name, lhs);
                link(&mut s, &name, rhs);
            }
            Phi {
                region, lhs, rhs, ..
            } => {
                link(&mut s, &name, region);
                link(&mut s, &name, lhs);
                link(&mut s, &name, rhs);
            }
        }
    }
    close(&mut s);
    s
}
