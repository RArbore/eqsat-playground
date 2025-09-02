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
    IfElse(ExpressionAST<'a>, BlockAST<'a>, Option<BlockAST<'a>>),
    While(ExpressionAST<'a>, BlockAST<'a>),
    Return(ExpressionAST<'a>),
}

#[derive(Debug)]
pub enum ExpressionAST<'a> {
    NumberLiteral(i32),
    Variable(IdentifierId),

    Call(IdentifierId, ArenaVec<'a, ExpressionAST<'a>>),

    Add(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    Subtract(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    Multiply(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    Divide(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    Modulo(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),

    EqualsEquals(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    NotEquals(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    Less(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    LessEquals(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    Greater(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
    GreaterEquals(&'a ExpressionAST<'a>, &'a ExpressionAST<'a>),
}

impl Default for StatementAST<'_> {
    fn default() -> Self {
        Self::Block(BlockAST::default())
    }
}

impl Default for ExpressionAST<'_> {
    fn default() -> Self {
        Self::NumberLiteral(0)
    }
}

#[cfg(test)]
mod tests {
    use util::arena::Arena;
    use util::interner::StringInterner;

    use crate::grammar::ProgramParser;

    fn get_example_imp_programs() -> Vec<String> {
        let var = "CARGO_MANIFEST_DIR";
        let val = std::env::var(var).unwrap();
        let mut path = std::path::PathBuf::from(val);
        path.push("examples");
        let mut programs = vec![];
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            programs.push(std::fs::read_to_string(entry.path()).unwrap());
        }
        programs
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn parse() {
        let mut string_buf: [u8; 100] = [0; 100];
        let string_arena = Arena::new_backed(&mut string_buf);
        let mut interner = StringInterner::new(&string_arena);

        let mut buf: [u8; 10000] = [0; 10000];
        let arena = Arena::new_backed(&mut buf);

        for program in get_example_imp_programs() {
            ProgramParser::new()
                .parse(&arena, &mut interner, &program)
                .unwrap();
        }
    }
}
