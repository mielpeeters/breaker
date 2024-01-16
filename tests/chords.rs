use breaker::{
    grid::GridToken,
    pipeline::{Pipeline, Playable},
};
use tree_sitter::Parser;

fn get_test_tree() -> (String, tree_sitter::Tree) {
    let source = include_str!("../testdata/simple_chord.br");

    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_breaker::language())
        .unwrap();

    let tree = parser.parse(&source, None).unwrap();

    (source.to_string(), tree)
}

#[test]
fn chord_parse_and_freqs() {
    let (source, tree) = get_test_tree();

    let playables = Pipeline::from_tree(&tree, &source, None).0.playables;

    let grid = playables
        .iter()
        .find(|p| p.0 == "chordName")
        .expect("chordName not found")
        .1;

    let chord = match grid {
        Playable::Grid(g) => g.tokens.first().unwrap(),
    };

    match chord {
        GridToken::Chord(c) => {
            let freqs = c.to_freqs();
            assert!(freqs.len() == 3, "Chord length is not 3, but {:?}", freqs);
        }
        _ => panic!("GridToken is not a chord"),
    }
}
