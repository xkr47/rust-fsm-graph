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

    let green = "\"#008000\"";

    let mut dot = format!(r#"digraph "graph" {{
  rankdir="LR";
  newrank=true;
  SM_init [label="", shape=point];
  SM_init -> "{}";

  subgraph "cluster_legend" {{
    label="Legend";
    __init [ shape=point ];
    __init -> __init2;
    __init2 [ shape=none label="Initial transition" ];
    __state [ label="state" ];
    __input [ label="input" color={} shape=cds ];
    __output [ label="output" color=red shape=note ];
  }}

"#,
                          fsm.initial_state.to_string(),
                          green);

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
                        // with "action"

                        let output_node = format!("{}_{}", o, final_state);
                        let iv_node = format!("{}_{}_{}_iv", o, final_state, input_values.join("_"));

                        // reason I use arrowhead, arrowtail here + reversed order of nodes is so that they rank frim right-to-left instead of left-to-right

                        dot2.insert(format!("  \"{}\" -> \"{}\" [ arrowhead=none arrowtail=normal style=dashed dir=both ];\n",
                                            iv_node,
                                            from_state));


                        if from_state == final_state {
                            dot2.insert(format!("  {{ rank=same; \"{}\"; \"{}\"; }}\n", from_state, iv_node));
                        }

                        dot2.insert(format!("  \"{}\" [label=\"{}\" color={} shape=cds ];\n",
                                            iv_node,
                                            input_values.join(",\n"),
                                            green));

                        dot2.insert(format!("  \"{}\" -> \"{}\" [ color={} arrowhead=none arrowtail=normal dir=both ];\n",
                                            output_node,
                                            iv_node,
                                            green));


                        dot2.insert(format!("  \"{}\" [label=\"{}\" color=red shape=note ];\n",
                                            output_node,
                                            o));

                        dot2.insert(format!("  \"{}\" -> \"{}\" [ color=red arrowhead=none arrowtail=normal dir=both ];\n",
                                            final_state,
                                            output_node,
                        ));
                    } else {
                        // without "action"

                        let iv_node = format!("{}_{}_iv", from_state, final_state);

                        if from_state == final_state {
                            dot2.insert(format!("  {{ rank=same; \"{}\"; \"{}\"; }}\n", from_state, iv_node));
                        }

                        dot2.insert(format!("  \"{}\" -> \"{}\" [ style=dashed rankdir=TB ];\n",
                                            from_state, iv_node
                                            ));


                        dot2.insert(format!("  \"{}\" [label=\"{}\" color={} shape=cds ];\n",
                                            iv_node,
                                            input_values.iter().join(",\n"),
                                            green));

                        dot2.insert(format!("  \"{}\" -> \"{}\" [ color={} ];\n",
                                            iv_node,
                                            final_state,
                                            green));
                    }
                });
        });

    dot2.iter().for_each(|line| dot.push_str(&line));

    dot.push_str("}\n");

    (name, dot)
}
