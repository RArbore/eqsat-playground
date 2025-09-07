use core::cell::RefCell;
use std::collections::HashMap;

use util::interner::{IdentifierId, StringInterner};

use crate::ast::{BlockAST, ExpressionAST, ProgramAST, StatementAST};
use crate::interval::IntervalDomain;
use crate::ssa::{Graph, SSADomain, Term};

pub trait AbstractDomain: Clone + PartialEq {
    type Value;

    fn interp_expr(&self, expr: &ExpressionAST<'_>) -> Self::Value;
    fn get(&self, iden: IdentifierId) -> Self::Value;
    fn assign(&mut self, iden: IdentifierId, val: Self::Value);
    fn branch(&self, cond: Self::Value) -> (Self, Self);
    fn finish_with(&mut self, val: Self::Value);
    fn join(&self, other: &Self) -> Self;
    fn widen(&self, other: &Self) -> (Self, bool);
}

pub fn abstract_interpret(program: &ProgramAST<'_>, interner: &mut StringInterner) -> Vec<Graph> {
    let mut graphs = vec![];
    for func in program.funcs.as_ref() {
        let mut graph = Graph::new(interner);
        let start = graph.makeset();
        graph.insert(Term::Start { root: start });
        let mut params = vec![];
        let mut param_idens = vec![];
        for (idx, iden) in func.params.as_ref().into_iter().enumerate() {
            let root = graph.makeset();
            graph.insert(Term::Param {
                start,
                index: idx as u32,
                root,
            });
            params.push((*iden, graph.find(root)));
            param_idens.push(*iden);
        }

        let interval = IntervalDomain::new(param_idens);
        println!("{:?}", ai_block(&func.block, &interval));

        let graph = RefCell::new(graph);
        let static_phis = RefCell::new(HashMap::new());
        let domain = SSADomain::new(&graph, &static_phis, start, params);
        ai_block(&func.block, &domain);
        graphs.push(graph.into_inner());
    }
    graphs
}

fn ai_block<AD: AbstractDomain>(block: &BlockAST<'_>, ad: &AD) -> AD {
    let mut ad = ad.clone();
    for stmt in block.stmts.as_ref() {
        ad = ai_stmt(stmt, &ad);
    }
    ad
}

fn ai_stmt<AD: AbstractDomain>(stmt: &StatementAST<'_>, ad: &AD) -> AD {
    use StatementAST::*;
    match stmt {
        Block(block) => ai_block(block, ad),
        Assign(iden, expr) => {
            let mut ad = ad.clone();
            let value = ad.interp_expr(expr);
            ad.assign(*iden, value);
            ad
        }
        IfElse(cond, lhs, rhs) => {
            let value = ad.interp_expr(cond);
            let (true_ad, mut false_ad) = ad.branch(value);
            let true_ad = ai_block(lhs, &true_ad);
            if let Some(rhs) = rhs {
                false_ad = ai_block(rhs, &false_ad);
            }
            true_ad.join(&false_ad)
        }
        While(cond, body) => {
            let mut iter = ad.clone();
            loop {
                let (top, widening) = ad.widen(&iter);
                let cond = top.interp_expr(cond);
                let (cont, exit) = top.branch(cond);
                let bottom = ai_block(body, &cont);
                if bottom == iter && !widening {
                    break exit;
                } else {
                    iter = bottom;
                }
            }
        }
        Return(expr) => {
            let mut ad = ad.clone();
            let value = ad.interp_expr(expr);
            ad.finish_with(value);
            ad
        }
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
            "cons([4294967295]) -> [6]\nparam([0, 0]) -> [1]\nstart([]) -> [0]\nregion([0, 4]) -> [2]\nregion([0, 11]) -> [2]\nbranch([2, 1]) -> [3]\nbranch([2, 8]) -> [10]\nπ([3, 1]) -> [4]\nπ([3, 0]) -> [5]\nπ([10, 1]) -> [11]\nπ([10, 0]) -> [12]\nfinish([12, 8]) -> [27]\nϕ([2, 1, 7]) -> [9]\nϕ([2, 1, 14]) -> [8]\n+([1, 6]) -> [7]\n+([8, 6]) -> [14]\n",
        );
    }
}
