use std::collections::BTreeMap;

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

type AbstractStore = BTreeMap<IdentifierId, ClassId>;

fn ai_func(func: &FunctionAST<'_>, graph: &mut Graph) {
    let mut s = AbstractStore::new();
    for (idx, iden) in func.params.as_ref().into_iter().enumerate() {
        let root = graph.makeset();
        graph.insert(Term::Param {
            index: idx as u32,
            root,
        });
        s.insert(*iden, graph.find(root));
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
    let merge_down = |lhs_s: &AbstractStore, rhs_s: &AbstractStore, graph: &mut Graph| {
        let mut merged_s = AbstractStore::new();
        for (lhs_iden, lhs_expr) in lhs_s {
            if let Some(rhs_expr) = rhs_s.get(&lhs_iden) {
                if *lhs_expr == *rhs_expr {
                    merged_s.insert(*lhs_iden, *lhs_expr);
                } else {
                    let root = graph.makeset();
                    graph.insert(Term::Phi {
                        lhs: *lhs_expr,
                        rhs: *rhs_expr,
                        root,
                    });
                    merged_s.insert(*lhs_iden, graph.find(root));
                }
            }
        }
        merged_s
    };

    use StatementAST::*;
    match stmt {
        Block(block) => ai_block(block, in_s, graph),
        Assign(iden, expr) => {
            let expr = ai_expr(expr, in_s, graph);
            let mut s = in_s.clone();
            s.insert(*iden, expr);
            s
        }
        IfElse(cond, lhs, rhs) => {
            ai_expr(cond, in_s, graph);
            let lhs_s = ai_block(lhs, in_s, graph);
            let rhs_s = if let Some(rhs) = rhs {
                &ai_block(rhs, in_s, graph)
            } else {
                in_s
            };
            merge_down(&lhs_s, &rhs_s, graph)
        }
        While(cond, body) => {
            ai_expr(cond, in_s, graph);
            let mut after_body_s = ai_block(body, in_s, graph);
            let mut last_concrete_expr = AbstractStore::new();
            let mut static_phis = AbstractStore::new();
            let mut changed = true;
            loop {
                let mut before_body_s = merge_down(in_s, &after_body_s, graph);
                if !changed {
                    for (iden, static_phi) in static_phis {
                        graph.merge(static_phi, last_concrete_expr[&iden]);
                    }
                    after_body_s = ai_block(body, &before_body_s, graph);
                    break;
                } else {
                    changed = false;
                    for (iden, new_expr) in before_body_s.iter_mut() {
                        if let Some(old_expr) = after_body_s.get(iden)
                            && *new_expr != *old_expr
                        {
                            if let Some(concrete_expr) = last_concrete_expr.insert(*iden, *new_expr)
                            {
                                changed = concrete_expr != *new_expr || changed;
                            } else {
                                let static_phi = graph.makeset();
                                *new_expr = static_phi;
                                static_phis.insert(*iden, static_phi);
                                changed = true;
                            }
                        }
                        if let Some(static_phi) = static_phis.get(iden) {
                            *new_expr = *static_phi;
                        }
                    }
                    after_body_s = ai_block(body, &before_body_s, graph);
                }
            }
            merge_down(in_s, &after_body_s, graph)
        }
        Return(expr) => {
            ai_expr(expr, in_s, graph);
            in_s.clone()
        }
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
            graph.find(root)
        }
        Variable(iden) => *in_s.get(iden).unwrap(),
        Add(lhs, rhs) => {
            let lhs = ai_expr(lhs, in_s, graph);
            let rhs = ai_expr(rhs, in_s, graph);
            let root = graph.makeset();
            graph.insert(Term::Add { lhs, rhs, root });
            graph.find(root)
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

        let program = "fn basic(x, y) { z = x + y; while z { z = z + -1; } if z { z = z + z; } else { z = z + 5; } return z; }";
        let program = ProgramParser::new()
            .parse(&arena, &mut interner, &program)
            .unwrap();
        let mut graphs = abstract_interpret(&program, &mut interner);
        graphs[0].rebuild();
        assert_eq!(
            graphs[0].dump(&interner),
            "cons([4294967295]) -> [3]\ncons([5]) -> [20]\nparam([0]) -> [0]\nparam([1]) -> [1]\nϕ([2, 4]) -> [5]\nϕ([19, 21]) -> [22]\nϕ([2, 8]) -> [6]\n+([0, 1]) -> [2]\n+([2, 3]) -> [4]\n+([6, 3]) -> [8]\n+([6, 6]) -> [19]\n+([6, 20]) -> [21]\n",
        );
    }
}
