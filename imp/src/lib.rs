use lalrpop_util::lalrpop_mod;

pub mod ai;
pub mod ast;
pub mod interval;
pub mod ssa;

lalrpop_mod!(pub grammar);
