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

#[derive(Clone)]
struct AbstractState {
    vars: BTreeMap<IdentifierId, ClassId>,
    pred: ClassId,
}

impl AbstractState {
    fn new(start: ClassId) -> Self {
        Self {
            vars: BTreeMap::new(),
            pred: start,
        }
    }

    fn insert(&mut self, iden: IdentifierId, value: ClassId) -> Option<ClassId> {
        self.vars.insert(iden, value)
    }

    fn get(&self, iden: &IdentifierId) -> Option<&ClassId> {
        self.vars.get(iden)
    }

    fn iter(&self) -> impl Iterator<Item = (&IdentifierId, &ClassId)> {
        self.vars.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = (&IdentifierId, &mut ClassId)> {
        self.vars.iter_mut()
    }

    fn pred(&self) -> ClassId {
        self.pred
    }

    fn set_pred(&mut self, pred: ClassId) {
        self.pred = pred;
    }
}

fn ai_func(func: &FunctionAST<'_>, graph: &mut Graph) {
    let start = graph.makeset();
    graph.insert(Term::Start { root: start });
    let mut s = AbstractState::new(start);
    for (idx, iden) in func.params.as_ref().into_iter().enumerate() {
        let root = graph.makeset();
        graph.insert(Term::Param {
            start,
            index: idx as u32,
            root,
        });
        s.insert(*iden, graph.find(root));
    }
    ai_block(&func.block, &s, graph);
}

fn ai_block(block: &BlockAST<'_>, in_s: &AbstractState, graph: &mut Graph) -> AbstractState {
    let mut s = in_s.clone();
    for stmt in block.stmts.as_ref() {
        s = ai_stmt(stmt, &s, graph);
    }
    s
}

fn ai_stmt(stmt: &StatementAST<'_>, in_s: &AbstractState, graph: &mut Graph) -> AbstractState {
    let merge_down =
        |region: ClassId, lhs_s: &AbstractState, rhs_s: &AbstractState, graph: &mut Graph| {
            let mut merged_s = AbstractState::new(region);
            for (lhs_iden, lhs_expr) in lhs_s.iter() {
                if let Some(rhs_expr) = rhs_s.get(&lhs_iden) {
                    if *lhs_expr == *rhs_expr {
                        merged_s.insert(*lhs_iden, *lhs_expr);
                    } else {
                        let root = graph.makeset();
                        graph.insert(Term::Phi {
                            region,
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
            let cond = ai_expr(cond, in_s, graph);
            let pred = in_s.pred();
            let mut root = graph.makeset();
            graph.insert(Term::Branch { pred, cond, root });

            root = graph.find(root);
            let lhs_root = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: root,
                index: 1,
                root: lhs_root,
            });
            let mut lhs_s = in_s.clone();
            lhs_s.set_pred(lhs_root);
            let lhs_s = ai_block(lhs, &lhs_s, graph);

            let rhs_root = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: root,
                index: 0,
                root: rhs_root,
            });
            let mut rhs_s = in_s.clone();
            rhs_s.set_pred(rhs_root);
            let rhs_s = if let Some(rhs) = rhs {
                &ai_block(rhs, &rhs_s, graph)
            } else {
                &rhs_s
            };

            let root = graph.makeset();
            graph.insert(Term::Region {
                lhs: lhs_s.pred(),
                rhs: rhs_s.pred(),
                root,
            });
            merge_down(root, &lhs_s, &rhs_s, graph)
        }
        While(cond, body) => {
            let loop_cond_region = graph.makeset();
            let loop_cond_branch = graph.makeset();
            let loop_cond_proj_true = graph.makeset();
            let loop_cond_proj_false = graph.makeset();

            let mut prev_state = None;
            let mut static_phis = BTreeMap::new();
            let mut last_exprs = BTreeMap::new();
            let mut changed = true;
            let (cond, bottom_pred, break_s) = loop {
                let mut top_s = if let Some(ref prev_state) = prev_state {
                    changed = false;
                    let mut merged_s = merge_down(loop_cond_region, in_s, &prev_state, graph);
                    for (iden, merged_expr) in merged_s.iter_mut() {
                        if let Some(old_expr) = prev_state.get(iden)
                            && *merged_expr != *old_expr
                        {
                            if let Some(last_expr) = last_exprs.insert(*iden, *merged_expr) {
                                changed = changed || last_expr != *merged_expr;
                            }

                            if !static_phis.contains_key(iden) {
                                let static_phi = graph.makeset();
                                static_phis.insert(*iden, static_phi);
                                changed = true;
                            }
                            *merged_expr = static_phis[&iden];
                        }
                    }
                    merged_s
                } else {
                    in_s.clone()
                };

                if !changed {
                    for (iden, static_phi) in static_phis {
                        graph.merge(static_phi, last_exprs[&iden]);
                    }
                    top_s.set_pred(loop_cond_branch);
                    let cond = ai_expr(cond, &top_s, graph);
                    top_s.set_pred(loop_cond_proj_false);
                    break (cond, prev_state.unwrap().pred(), top_s);
                }

                top_s.set_pred(loop_cond_proj_true);
                let bottom_s = ai_block(body, &top_s, graph);
                prev_state = Some(bottom_s);
            };

            graph.insert(Term::Region {
                lhs: in_s.pred(),
                rhs: bottom_pred,
                root: loop_cond_region,
            });
            graph.insert(Term::Branch {
                pred: loop_cond_region,
                cond,
                root: loop_cond_branch,
            });
            graph.insert(Term::ControlProj {
                pred: loop_cond_branch,
                index: 0,
                root: loop_cond_proj_false,
            });
            graph.insert(Term::ControlProj {
                pred: loop_cond_branch,
                index: 1,
                root: loop_cond_proj_true,
            });
            break_s
        }
        Return(expr) => {
            let value = ai_expr(expr, in_s, graph);
            let root = graph.makeset();
            graph.insert(Term::Finish {
                pred: in_s.pred(),
                value,
                root,
            });
            in_s.clone()
        }
    }
}

fn ai_expr(expr: &ExpressionAST<'_>, in_s: &AbstractState, graph: &mut Graph) -> ClassId {
    use ExpressionAST::*;
    match expr {
        NumberLiteral(value) => {
            graph.constant(*value)
        }
        Variable(iden) => *in_s.get(iden).unwrap(),
        Add(lhs, rhs) => {
            let lhs = ai_expr(lhs, in_s, graph);
            let rhs = ai_expr(rhs, in_s, graph);
            graph.add(lhs, rhs)
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

        let program = "fn basic(x) { while x { x = x + -1; } return x; }";
        let program = ProgramParser::new()
            .parse(&arena, &mut interner, &program)
            .unwrap();
        let mut graphs = abstract_interpret(&program, &mut interner);
        graphs[0].rebuild();
        assert_eq!(
            graphs[0].dump(&interner),
            "cons([4294967295]) -> [6]\nparam([0, 0]) -> [1]\nstart([]) -> [0]\nregion([0, 4]) -> [2]\nbranch([2, 9]) -> [3]\nπ([3, 0]) -> [5]\nπ([3, 1]) -> [4]\nfinish([5, 9]) -> [16]\nϕ([2, 1, 7]) -> [8]\nϕ([2, 1, 11]) -> [9]\n+([1, 6]) -> [7]\n+([9, 6]) -> [11]\n",
        );
    }
}
