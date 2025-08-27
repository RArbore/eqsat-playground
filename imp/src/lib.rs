use lalrpop_util::lalrpop_mod;

pub mod ast;
lalrpop_mod!(grammar);

#[cfg(test)]
mod tests {
    use util::arena::Arena;
    use util::interner::StringInterner;

    use super::grammar::ProgramParser;

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
    fn parse() {
        let mut string_buf: [u8; 100] = [0; 100];
        let string_arena = Arena::new_backed(&mut string_buf);
        let mut interner = StringInterner::new(&string_arena);

        let mut buf: [u8; 1000] = [0; 1000];
        let arena = Arena::new_backed(&mut buf);

        for program in get_example_imp_programs() {
            ProgramParser::new().parse(&arena, &mut interner, &program).unwrap();
        }
    }
}
