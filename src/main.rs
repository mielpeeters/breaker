use breaker::{chromatic::Chord, grid::Grid, util::FromNode};
use clap::Parser as ClapParser;
use tree_sitter::Parser;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_file: String,
}

fn main() {
    let args = Args::parse();
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_breaker::language())
        .unwrap();

    // read file
    let source_code = std::fs::read_to_string(args.input_file).unwrap();

    // parse
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();

    println!("root: {}", root.to_sexp());

    let mut walk = root.walk();

    // TODO: find a way to traverse the tree properly
    //       how is this done in well-known parsers?
    let chord = loop {
        // go to the next sibling, else go one level deeper
        if !walk.goto_first_child() {
            if !walk.goto_next_sibling() {
                break None;
            }
        }

        if walk.node().kind() == "chord" {
            break Some(walk.node());
        }
    };

    let Some(chord) = chord else {
        println!("No chord found");
        return;
    };
    let chord = Chord::from_node(&chord, &source_code).unwrap();
    println!("chord: {}", chord);

    let root = tree.root_node();

    println!("root: {}", root.to_sexp());

    let mut walk = root.walk();

    let grid = loop {
        // go to the next sibling, else go one level deeper
        if !walk.goto_first_child() {
            if !walk.goto_next_sibling() {
                break None;
            }
        }

        if walk.node().kind() == "grid" {
            break Some(walk.node());
        }
    };

    let Some(grid) = grid else {
        println!("No grid found");
        return;
    };
    let grid = Grid::from_node(&grid, &source_code).unwrap();

    println!("grid: {}", grid);
}
