use core::cell::Cell;
use std::collections::BTreeMap;

use util::interner::{IdentifierId, StringInterner};
use util::union_find::ClassId;

use crate::ast::{BlockAST, ExpressionAST, FunctionAST, ProgramAST, StatementAST};
use crate::ssa::{Graph, Term};

struct ControlLinker {
    last: Cell<ClassId>,
}

impl ControlLinker {
    fn new(id: ClassId) -> Self {
        Self {
            last: Cell::new(id),
        }
    }

    fn get(&self) -> ClassId {
        self.last.get()
    }

    fn set(&self, id: ClassId) {
        self.last.set(id);
    }
}

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
    let root = graph.makeset();
    graph.insert(Term::Start { root });
    let ctrl = ControlLinker::new(root);
    let mut s = AbstractStore::new();
    for (idx, iden) in func.params.as_ref().into_iter().enumerate() {
        let root = graph.makeset();
        graph.insert(Term::Param {
            index: idx as u32,
            root,
        });
        s.insert(*iden, graph.find(root));
    }
    ai_block(&func.block, &s, graph, &ctrl);
}

fn ai_block(
    block: &BlockAST<'_>,
    in_s: &AbstractStore,
    graph: &mut Graph,
    ctrl: &ControlLinker,
) -> AbstractStore {
    let mut s = in_s.clone();
    for stmt in block.stmts.as_ref() {
        s = ai_stmt(stmt, &s, graph, ctrl);
    }
    s
}

fn ai_stmt(
    stmt: &StatementAST<'_>,
    in_s: &AbstractStore,
    graph: &mut Graph,
    ctrl: &ControlLinker,
) -> AbstractStore {
    let merge_down =
        |region: ClassId, lhs_s: &AbstractStore, rhs_s: &AbstractStore, graph: &mut Graph| {
            let mut merged_s = AbstractStore::new();
            for (lhs_iden, lhs_expr) in lhs_s {
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
        Block(block) => ai_block(block, in_s, graph, ctrl),
        Assign(iden, expr) => {
            let expr = ai_expr(expr, in_s, graph, ctrl);
            let mut s = in_s.clone();
            s.insert(*iden, expr);
            s
        }
        IfElse(cond, lhs, rhs) => {
            let cond = ai_expr(cond, in_s, graph, ctrl);
            let pred = ctrl.get();
            let mut root = graph.makeset();
            graph.insert(Term::Branch { pred, cond, root });
            root = graph.find(root);
            let mut lhs_root = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: root,
                index: 1,
                root: lhs_root,
            });
            let mut rhs_root = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: root,
                index: 0,
                root: rhs_root,
            });

            ctrl.set(lhs_root);
            let lhs_s = ai_block(lhs, in_s, graph, ctrl);
            lhs_root = ctrl.get();
            ctrl.set(rhs_root);
            let rhs_s = if let Some(rhs) = rhs {
                &ai_block(rhs, in_s, graph, ctrl)
            } else {
                in_s
            };
            rhs_root = ctrl.get();
            let root = graph.makeset();
            graph.insert(Term::Region {
                lhs: lhs_root,
                rhs: rhs_root,
                root,
            });
            ctrl.set(root);
            merge_down(root, &lhs_s, &rhs_s, graph)
        }
        While(cond, body) => {
            let loop_body_region = graph.makeset();
            let loop_exit_region = graph.makeset();
            let loop_body_branch = graph.makeset();
            let loop_entry_proj = graph.makeset();
            let entry_pred = ctrl.get();
            ctrl.set(loop_entry_proj);
            let original_cond_expr = ai_expr(cond, in_s, graph, ctrl);
            ctrl.set(loop_body_region);
            let mut after_body_s = ai_block(body, in_s, graph, ctrl);
            let mut last_concrete_expr = AbstractStore::new();
            let mut static_phis = AbstractStore::new();
            let mut changed = true;
            let (loop_body_bottom, cond_expr) = loop {
                let mut before_body_s = merge_down(loop_body_region, in_s, &after_body_s, graph);
                if !changed {
                    for (iden, static_phi) in static_phis {
                        graph.merge(static_phi, last_concrete_expr[&iden]);
                    }
                    ctrl.set(loop_body_region);
                    after_body_s = ai_block(body, &before_body_s, graph, ctrl);
                    break (ctrl.get(), ai_expr(cond, &after_body_s, graph, ctrl));
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
                    ctrl.set(loop_body_region);
                    after_body_s = ai_block(body, &before_body_s, graph, ctrl);
                    ai_expr(cond, &after_body_s, graph, ctrl);
                }
            };
            let loop_entry_branch = graph.makeset();
            graph.insert(Term::Branch {
                pred: entry_pred,
                cond: original_cond_expr,
                root: loop_entry_branch,
            });
            let loop_skip_proj = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: loop_entry_branch,
                index: 0,
                root: loop_skip_proj,
            });
            graph.insert(Term::ControlProj {
                pred: loop_entry_branch,
                index: 1,
                root: loop_entry_proj,
            });
            graph.insert(Term::Branch {
                pred: loop_body_bottom,
                cond: cond_expr,
                root: loop_body_branch,
            });
            let loop_body_done_proj = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: loop_body_branch,
                index: 0,
                root: loop_body_done_proj,
            });
            let loop_body_continue_proj = graph.makeset();
            graph.insert(Term::ControlProj {
                pred: loop_body_branch,
                index: 1,
                root: loop_body_continue_proj,
            });
            graph.insert(Term::Region {
                lhs: loop_entry_proj,
                rhs: loop_body_continue_proj,
                root: loop_body_region,
            });
            graph.insert(Term::Region {
                lhs: loop_skip_proj,
                rhs: loop_body_done_proj,
                root: loop_exit_region,
            });
            ctrl.set(loop_exit_region);
            merge_down(loop_exit_region, in_s, &after_body_s, graph)
        }
        Return(expr) => {
            let value = ai_expr(expr, in_s, graph, ctrl);
            let root = graph.makeset();
            graph.insert(Term::Finish {
                pred: ctrl.get(),
                value,
                root,
            });
            in_s.clone()
        }
    }
}

fn ai_expr(
    expr: &ExpressionAST<'_>,
    in_s: &AbstractStore,
    graph: &mut Graph,
    ctrl: &ControlLinker,
) -> ClassId {
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
            let lhs = ai_expr(lhs, in_s, graph, ctrl);
            let rhs = ai_expr(rhs, in_s, graph, ctrl);
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

        let program = "fn basic(x) { while x { x = x + -1; } return x; }";
        let program = ProgramParser::new()
            .parse(&arena, &mut interner, &program)
            .unwrap();
        let mut graphs = abstract_interpret(&program, &mut interner);
        graphs[0].rebuild();
        assert_eq!(
            graphs[0].dump(&interner),
            "cons([4294967295]) -> [6]\nparam([0]) -> [1]\nstart([]) -> [0]\nregion([5, 24]) -> [2]\nregion([22, 23]) -> [3]\nbranch([0, 1]) -> [21]\nbranch([2, 11]) -> [4]\nπ([21, 0]) -> [22]\nπ([21, 1]) -> [5]\nπ([4, 0]) -> [23]\nπ([4, 1]) -> [24]\nfinish([3, 25]) -> [26]\nϕ([2, 1, 7]) -> [8]\nϕ([2, 1, 11]) -> [9]\nϕ([3, 1, 11]) -> [25]\n+([1, 6]) -> [7]\n+([9, 6]) -> [11]\n",
        );
    }
}
