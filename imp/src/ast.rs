use util::interner::IdentifierId;
use util::vec::ArenaVec;

#[derive(Debug, Default)]
pub struct ProgramAST<'a> {
    pub funcs: ArenaVec<'a, FunctionAST<'a>>,
}

#[derive(Debug, Default)]
pub struct FunctionAST<'a> {
    pub name: IdentifierId,
    pub params: ArenaVec<'a, IdentifierId>,
    pub block: BlockAST<'a>,
}

#[derive(Debug, Default)]
pub struct BlockAST<'a> {
    pub stmts: ArenaVec<'a, StatementAST<'a>>,
}

#[derive(Debug)]
pub enum StatementAST<'a> {
    Block(BlockAST<'a>),
    Assign(IdentifierId, ExpressionAST<'a>),
    Return(ExpressionAST<'a>),
}

#[derive(Debug)]
pub enum ExpressionAST<'a> {
    NumberLiteral(i64),
    Variable(IdentifierId),
    Add(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
}

impl Default for StatementAST<'_> {
    fn default() -> Self {
        Self::Block(BlockAST::default())
    }
}
