use std::env;
use std::fs::File;
use std::io::Read;
use std::process;

use proc_macro2::TokenStream;
use syn::{Item, parse2};

use crate::parser::StateMachineDef;

mod parser;

fn main() {
    let mut args = env::args();
    let _ = args.next(); // executable name

    let filename = match (args.next(), args.next()) {
        (Some(filename), None) => filename,
        _ => {
            eprintln!("Usage: rust-fsm-graph path/to/filename.rs");
            process::exit(1);
        }
    };

    let mut file = File::open(&filename).expect("Unable to open file");

    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");

    let syntax = syn::parse_file(&src).expect("Unable to parse file");

    syntax.items.into_iter()
        .filter_map(|item|
                    if let Item::Macro(item_macro) = item {
                        let seg = &item_macro.mac.path.segments;
                        if seg.len() == 1 && seg.first().unwrap().ident == "state_machine" {
                            Some(item_macro.mac.tokens)
                        } else {
                            None
                        }
                    } else {
                        None
                    })
        .map(fsm_to_graphviz)
        .for_each(|(name, str)| {
            let filename = format!("{}.dot", name);
            std::fs::write(&filename, str).unwrap_or_else(|_| panic!("Failed to write {}", &filename));
            println!("Wrote {}", filename);
        });
}

fn fsm_to_graphviz(stream: TokenStream) -> (String, String) {
    let fsm = parse2::<StateMachineDef>(stream).expect("Failed to parse state machine definition");
    let name = fsm.name.to_string();

    let mut dot = format!(r#"digraph "graph" {{
  rankdir="LR";
  SM_init [label="", shape=point];
  SM_init -> "{}";
"#, fsm.initial_state.to_string());

    fsm.transitions.into_iter()
        .flat_map(|from| {
            let from_state = from.initial_state.to_string();
            from.transitions.into_iter()
                .map(move |to| format!("  \"{}\" -> \"{}\" [label=\"{}{}\" minlen=2];\n",
                                       from_state,
                                       to.final_state.to_string(),
                                       to.input_value.to_string(),
                                       to.output.map_or(String::new(), |v| format!(" [{}]", v.to_string()))))
        })
        .for_each(|line| dot.push_str(&line));

    dot.push_str("}\n");

    (name, dot)
}
