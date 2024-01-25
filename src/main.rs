use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use breaker::{
    audio_engine,
    pipeline::{Pipeline, PipelineConfig},
};
use clap::Parser as ClapParser;
use notify::{
    event::{DataChange, ModifyKind},
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use tree_sitter::Parser;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(required = true)]
    input_file: String,

    #[arg(short, long, default_value = "samples")]
    sample_dir: String,
}

fn main() {
    let args = Args::parse();
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_breaker::language())
        .unwrap();

    let input_file = PathBuf::try_from(args.input_file).unwrap();

    // read file
    let source_code = std::fs::read_to_string(&input_file).unwrap();

    // parse
    let mut tree = parser.parse(&source_code, None).unwrap();

    let pipeline_config = PipelineConfig {
        samples_dir: args.sample_dir,
    };

    let (pipeline, source) = Pipeline::from_tree(&tree, &source_code, Some(&pipeline_config));
    let shared_pipeline = Arc::new(Mutex::new(pipeline));
    let _eng = audio_engine::start(source);

    // run the pipeline thread
    let shared_pipeline_thread = shared_pipeline.clone();
    thread::spawn(move || loop {
        let mut p = shared_pipeline_thread.lock().unwrap();

        let Ok(_) = p.send_sample() else {
            break;
        };
    });

    // set up file watcher
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();

    watcher
        .watch(input_file.parent().unwrap(), RecursiveMode::Recursive)
        .unwrap();

    let target_event_kind = EventKind::Modify(ModifyKind::Data(DataChange::Any));

    for msg in rx {
        match msg {
            Ok(event) => {
                if event.kind != target_event_kind {
                    continue;
                }
                if event
                    .paths
                    .iter()
                    .any(|path| path.file_name() == input_file.file_name())
                {
                    // println!("Reparse the tree!");
                    let source_code = std::fs::read_to_string(&input_file).unwrap();
                    tree = parser.parse(&source_code, None).unwrap();
                    let new_p = Pipeline::from_tree(&tree, &source_code, Some(&pipeline_config)).0;
                    {
                        let mut p = shared_pipeline.lock().unwrap();
                        p.update(new_p);
                        // println!("Tree was updated!");
                    }
                    // TODO: update the audio pipeline in the <Arc<Mutex<Pipeline>> such that the
                    //       audio engine will be updated
                }
            }
            Err(err) => println!("Error: {}", err),
        }
    }
}
