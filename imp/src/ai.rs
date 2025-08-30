use std::collections::HashMap;

use util::interner::{IdentifierId, StringInterner};
use util::union_find::ClassId;

use crate::ast::{BlockAST, ExpressionAST, FunctionAST, ProgramAST, StatementAST};
use crate::ssa::{Graph, Term};

pub fn abstract_interpret(program: &ProgramAST<'_>, interner: &mut StringInterner) -> Vec<Graph> {
    let mut graphs = vec![];
    for func in program.funcs.as_ref() {
        let mut graph = Graph::new(interner);
        ai_func(func, &mut graph);
        graphs.push(graph);
    }
    graphs
}

type AbstractStore = HashMap<IdentifierId, ClassId>;

fn ai_func(func: &FunctionAST<'_>, graph: &mut Graph) {
    let mut s = AbstractStore::new();
    for (idx, iden) in func.params.as_ref().into_iter().enumerate() {
        let root = graph.makeset();
        graph.insert(Term::Param {
            index: idx as u32,
            root,
        });
        s.insert(*iden, root);
    }
    ai_block(&func.block, &s, graph);
}

fn ai_block(block: &BlockAST<'_>, in_s: &AbstractStore, graph: &mut Graph) -> AbstractStore {
    let mut s = in_s.clone();
    for stmt in block.stmts.as_ref() {
        s = ai_stmt(stmt, &s, graph);
    }
    s
}

fn ai_stmt(stmt: &StatementAST<'_>, in_s: &AbstractStore, graph: &mut Graph) -> AbstractStore {
    use StatementAST::*;
    match stmt {
        Block(block) => ai_block(block, in_s, graph),
        Assign(iden, expr) => {
            let expr = ai_expr(expr, in_s, graph);
            let mut s = in_s.clone();
            s.insert(*iden, expr);
            s
        }
        Return(expr) => {
            ai_expr(expr, in_s, graph);
            in_s.clone()
        }
        _ => todo!(),
    }
}

fn ai_expr(expr: &ExpressionAST<'_>, in_s: &AbstractStore, graph: &mut Graph) -> ClassId {
    use ExpressionAST::*;
    match expr {
        NumberLiteral(value) => {
            let root = graph.makeset();
            graph.insert(Term::Constant {
                value: *value,
                root,
            });
            root
        }
        Variable(iden) => *in_s.get(iden).unwrap(),
        Add(lhs, rhs) => {
            let lhs = ai_expr(lhs, in_s, graph);
            let rhs = ai_expr(rhs, in_s, graph);
            let root = graph.makeset();
            graph.insert(Term::Add { lhs, rhs, root });
            root
        }
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use util::arena::Arena;
    use util::interner::StringInterner;

    use crate::grammar::ProgramParser;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn simple_ai() {
        let mut string_buf: [u8; 100] = [0; 100];
        let string_arena = Arena::new_backed(&mut string_buf);
        let mut interner = StringInterner::new(&string_arena);
        let mut buf: [u8; 10000] = [0; 10000];
        let arena = Arena::new_backed(&mut buf);

        let program = "fn basic(x, y) { z = x + y; return z; }";
        let program = ProgramParser::new()
            .parse(&arena, &mut interner, &program)
            .unwrap();
        let graphs = abstract_interpret(&program, &mut interner);
        assert_eq!(
            graphs[0].dump(&interner),
            "param([0]) -> [0]\nparam([1]) -> [1]\n+([0, 1]) -> [2]\n"
        );
    }
}
