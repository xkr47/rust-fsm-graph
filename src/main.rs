use std::env;
use std::fs::read_to_string;
use std::process;

use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use syn::{Item, parse2};

use crate::parser::StateMachineDef;
use std::hash::Hash;
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;

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

    let green = "\"#00c000\"";

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

    let mut seen_states = HashSet::new();
    let mut dot2 = Vec::new();
    let mut seen_edges = LinkedHashSet::new();
    let line_styles = ["dashed", "dotted", "solid", "bold"].iter().map(|style| format!("style={}", style)).collect::<Vec<_>>();
    let mut line_styles = line_styles.iter().cycle();
    fsm.transitions.into_iter()
        .for_each(|from| {
            let from_state = from.initial_state.to_string();
            eprintln!("from {}", from_state);
            seen_states.insert(from_state.clone());
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
                .into_linked_group_map()
                .into_iter()
                .for_each(|((output, final_state), input_values)| {
                    eprintln!("  to {} output {:?} for inputs {:?}", final_state, output, input_values);
                    let reverse = from_state != final_state && seen_states.contains(&final_state);
                    let line_style = line_styles.next().unwrap();
                    if let Some(o) = output {
                        let output_node = format!("{}_{}", o, final_state);
                        let iv_node = format!("{}_{}_{}_iv", o, final_state, input_values.join("_"));

                        // reason I use arrowhead, arrowtail here + reversed order of nodes is so that they rank frim right-to-left instead of left-to-right

                        insert_edge(&mut dot2, &mut seen_edges, reverse, &from_state, &iv_node, line_style);


                        if from_state == final_state {
                            dot2.push(format!("  {{ rank=same; \"{}\"; \"{}\"; }}\n", from_state, iv_node));
                        }

                        dot2.push(format!("  \"{}\" [label=\"{}\" color={} shape=cds ];\n",
                                          iv_node,
                                          input_values.join(",\n"),
                                          green));

                        insert_edge(&mut dot2, &mut seen_edges, reverse,iv_node, &output_node, format!("{} color={}", line_style, green));


                        dot2.push(format!("  \"{}\" [label=\"{}\" color=red shape=note ];\n",
                                          output_node,
                                          o));

                        insert_edge(&mut dot2, &mut seen_edges, reverse,output_node, final_state, format!("{} color=red", line_style));

                    } else {
                        let iv_node = format!("{}_{}_iv", final_state, input_values.join("_"));

                        if from_state == final_state {
                            dot2.push(format!("  {{ rank=same; \"{}\"; \"{}\"; }}\n", from_state, iv_node));
                        }

                        insert_edge(&mut dot2, &mut seen_edges, reverse,&from_state, &iv_node, line_style);


                        dot2.push(format!("  \"{}\" [label=\"{}\" color={} shape=cds ];\n",
                                          iv_node,
                                          input_values.iter().join(",\n"),
                                          green));

                        insert_edge(&mut dot2, &mut seen_edges, reverse, iv_node, final_state, format!("{} color={}", line_style, green));
                    }
                });
        });

    dot2.iter().for_each(|line| dot.push_str(&line));

    dot.push_str("}\n");

    (name, dot)
}

fn insert_edge<F: AsRef<str>, T: AsRef<str>, S: AsRef<str>>(dot2: &mut Vec<String>, seen_edges: &mut LinkedHashSet<(String, String)>, reverse: bool, from: F, to: T, styles: S) {
    eprintln!("    {} - {} {}", from.as_ref(), to.as_ref(), if reverse { "rev" } else { "" });
    if !seen_edges.insert((from.as_ref().to_string(), to.as_ref().to_string())) {
        return;
    }
    if reverse {
        dot2.push(format!("  \"{}\" -> \"{}\" [ arrowhead=none arrowtail=normal dir=both {} ];\n",
                          to.as_ref(),
                          from.as_ref(),
                          styles.as_ref()));
    } else {
        dot2.push(format!("  \"{}\" -> \"{}\" [ {} ];\n",
                          from.as_ref(),
                          to.as_ref(),
                          styles.as_ref()));
    }
}

// based on https://github.com/rust-itertools/itertools/blob/master/src/group_map.rs
pub fn into_linked_group_map<I, K, V>(iter: I) -> LinkedHashMap<K, Vec<V>>
    where I: Iterator<Item=(K, V)>,
          K: Hash + Eq,
{
    let mut lookup = LinkedHashMap::new();

    for (key, val) in iter {
        lookup.entry(key).or_insert(Vec::new()).push(val);
    }

    lookup
}

pub trait ILGM : Iterator {
    fn into_linked_group_map<K, V>(self) -> LinkedHashMap<K, Vec<V>>
        where Self: Iterator<Item=(K, V)> + Sized,
              K: Hash + Eq,
    {
        into_linked_group_map(self)
    }
}

impl<T: ?Sized> ILGM for T where T: Iterator { }
