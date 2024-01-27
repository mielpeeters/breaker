use breakers::{
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

    let tree = parser.parse(source, None).unwrap();

    (source.to_string(), tree)
}

fn test_freqs(freqs: Vec<f64>, expected: &[f64]) {
    // first test lengths
    assert_eq!(
        freqs.len(),
        expected.len(),
        "Lengths don't match: expected {}, got {}",
        expected.len(),
        freqs.len()
    );

    // then test values
    freqs.iter().zip(expected.iter()).for_each(|(a, b)| {
        assert!(
            (a - b).abs() < 0.0001,
            "Expected {:?}, got {:?}",
            expected,
            freqs
        )
    });
}

#[test]
fn chord_parse_and_freqs() {
    let (source, tree) = get_test_tree();

    let playables = Pipeline::from_tree(&tree, &source, None).0.playables;

    let grid = playables
        .iter()
        .find(|p| {
            println!("Found this: {:?}", p);
            p.0 == "chordName"
        })
        .expect("chordName not found")
        .1;

    let chord = match grid {
        Playable::Grid(g) => g.tokens.first().unwrap(),
    };

    match chord {
        GridToken::Chord(c) => {
            let freqs = c.as_freqs();
            test_freqs(
                freqs,
                &[
                    493.8833012561241,
                    587.3295358348151,
                    739.9888454232688,
                    880.0,
                    554.3652619537442,
                    174.61411571650194,
                ],
            )
        }
        _ => panic!("GridToken is not a chord"),
    }
}
