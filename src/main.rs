use std::env;
use std::fs::read_to_string;
use std::process;

use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
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

    read_fsms(&filename)
        .into_iter()
        .map(fsm_to_graphviz)
        .for_each(|(name, str)| {
            let filename = format!("{}.dot", name);
            std::fs::write(&filename, str).unwrap_or_else(|_| panic!("Failed to write {}", &filename));
            println!("Wrote {}", filename);
        });
}


fn read_fsms(file: &str) -> Vec<StateMachineDef> {
    syn::parse_file(&read_to_string(file).expect("failed to read file"))
        .expect("Unable to parse file")
        .items.into_iter()
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
        .map(|ts| parse2::<StateMachineDef>(ts).expect("Failed to parse state machine definition"))
        .collect()
}

fn fsm_to_graphviz(fsm: StateMachineDef) -> (String, String) {
    let name = fsm.name.to_string();

    let mut dot = format!(r#"digraph "graph" {{
  rankdir="LR";
  node [shape=Mrecord];
  newrank=true;
  SM_init [label="", shape=point];
  SM_init -> "{}";
"#, fsm.initial_state.to_string());


    let mut dot2 = LinkedHashSet::new();
    fsm.transitions.into_iter()
        .for_each(|from| {
            let from_state = from.initial_state.to_string();
            from.transitions.into_iter()
                .map(|to| {
                    (
                        (
                            if let Some(i) = &to.output { Some(i.to_string()) } else { None },
                            to.final_state.to_string()
                        ),
                        to.input_value.to_string()
                    )
                })
                .into_group_map()
                .into_iter()
                .for_each(|((output, final_state), input_values)| {
                    if let Some(o) = output {
                        let output_node = format!("{}_{}", o, final_state);

                        dot2.insert(format!("  \"{}\" -> \"{}\" [label=\"{}\"  arrowhead=none arrowtail=normal dir=both ];\n",
                                            output_node,
                                            from_state,
                                            input_values.into_iter().join(",\n")));

                        dot2.insert(format!("  \"{}\" [label=\"{}\" color=red shape=note ];\n",
                                            output_node,
                                            o));
                        dot2.insert(format!("  \"{}\" -> \"{}\" [ color=red arrowhead=none arrowtail=normal dir=both ];\n",
                                            final_state,
                                            output_node,
                        ));
                    } else {
                        dot2.insert(format!("  \"{}\" -> \"{}\" [label=\"{}\" ];\n",
                                            from_state,
                                            final_state,
                                            input_values.into_iter().join(",\n")));

                    }
                });
        });

    dot2.iter().for_each(|line| dot.push_str(&line));

    dot.push_str("}\n");

    (name, dot)
}
