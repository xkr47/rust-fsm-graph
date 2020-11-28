use std::env;
use std::fs::File;
use std::io::Read;
use std::process;

use proc_macro2::TokenStream;
use syn::{Item, parse2};

use crate::parser::StateMachineDef;
use itertools::Itertools;

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
  node [shape=Mrecord];
  SM_init [label="", shape=point];
  SM_init -> "{}";
"#, fsm.initial_state.to_string());

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
                .map(move |((output, final_state), input_values)|
                    {
                        let is_multiple_to = input_values.len() > 1;
                        format!("  \"{}\" -> \"{}\" [label=\"{}{}\" minlen=2];\n",
                                from_state,
                                final_state,
                                join_squared(input_values),
                                output.map_or(String::new(), |v|
                                    format!("{}[{}]", if is_multiple_to { "\n" } else { " " }, v)))
                    })
                .for_each(|line| dot.push_str(&line));
        });

    dot.push_str("}\n");

    (name, dot)
}

fn join_squared(strings: Vec<String>) -> String {
    let tot = strings.len();
    let base_width = (tot as f32).sqrt() as usize;
    let height = if tot > (base_width + 1) * base_width { base_width + 1 } else { base_width };
    let extra = tot - base_width * height;
    println!("b {} {} {}", base_width, height, extra);
    let extra_start = (height - extra) / 2;
    let extra_end = extra_start + extra;
    println!("s {} {}", extra_start, extra_end);
    let src = strings.into_iter();
    let row_idxs = (0..height).flat_map(|y|
        vec![y; if extra_start <= y && y < extra_end { base_width + 1 } else { base_width }].into_iter()
    ).into_iter();
    let zip = src.zip(row_idxs);
    zip
        .group_by(|(_, row)| *row)
        .into_iter()
        .map(|(_, pairs)| pairs.map(|(str, _)| str).join(", "))
        .join("\n")
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    fn nums_to(e: i32) -> Vec<String> {
        (1..=e).map(|i| i.to_string()).collect()
    }

    #[test]
    fn test_join_squared() {
        assert_eq!(join_squared(nums_to(1)), "1");
        assert_eq!(join_squared(nums_to(2)), "1, 2");
        assert_eq!(join_squared(nums_to(3)), "1, 2\n3");
        assert_eq!(join_squared(nums_to(4)), "1, 2\n3, 4");
        assert_eq!(join_squared(nums_to(5)), "1, 2, 3\n4, 5");
        assert_eq!(join_squared(nums_to(6)), "1, 2, 3\n4, 5, 6");
        assert_eq!(join_squared(nums_to(7)), "1, 2\n3, 4, 5\n6, 7");
        assert_eq!(join_squared(nums_to(8)), "1, 2, 3\n4, 5, 6\n7, 8");
        assert_eq!(join_squared(nums_to(9)), "1, 2, 3\n4, 5, 6\n7, 8, 9");
        assert_eq!(join_squared(nums_to(10)), "1, 2, 3\n4, 5, 6, 7\n8, 9, 10");
        assert_eq!(join_squared(nums_to(11)), "1, 2, 3, 4\n5, 6, 7, 8\n9, 10, 11");
        assert_eq!(join_squared(nums_to(12)), "1, 2, 3, 4\n5, 6, 7, 8\n9, 10, 11, 12");
        assert_eq!(join_squared(nums_to(13)), "1, 2, 3\n4, 5, 6, 7\n8, 9, 10\n11, 12, 13");
        assert_eq!(join_squared(nums_to(14)), "1, 2, 3\n4, 5, 6, 7\n8, 9, 10, 11\n12, 13, 14");
        assert_eq!(join_squared(nums_to(15)), "1, 2, 3, 4\n5, 6, 7, 8\n9, 10, 11, 12\n13, 14, 15");
        assert_eq!(join_squared(nums_to(16)), "1, 2, 3, 4\n5, 6, 7, 8\n9, 10, 11, 12\n13, 14, 15, 16");
        assert_eq!(join_squared(nums_to(17)), "1, 2, 3, 4\n5, 6, 7, 8, 9\n10, 11, 12, 13\n14, 15, 16, 17");
        assert_eq!(join_squared(nums_to(18)), "1, 2, 3, 4\n5, 6, 7, 8, 9\n10, 11, 12, 13, 14\n15, 16, 17, 18");
        assert_eq!(join_squared(nums_to(19)), "1, 2, 3, 4, 5\n6, 7, 8, 9, 10\n11, 12, 13, 14, 15\n16, 17, 18, 19");
        assert_eq!(join_squared(nums_to(20)), "1, 2, 3, 4, 5\n6, 7, 8, 9, 10\n11, 12, 13, 14, 15\n16, 17, 18, 19, 20");
        assert_eq!(join_squared(nums_to(21)), "1, 2, 3, 4\n5, 6, 7, 8\n9, 10, 11, 12, 13\n14, 15, 16, 17\n18, 19, 20, 21");
        assert_eq!(join_squared(nums_to(22)), "1, 2, 3, 4\n5, 6, 7, 8, 9\n10, 11, 12, 13, 14\n15, 16, 17, 18\n19, 20, 21, 22");
        assert_eq!(join_squared(nums_to(23)), "1, 2, 3, 4\n5, 6, 7, 8, 9\n10, 11, 12, 13, 14\n15, 16, 17, 18, 19\n20, 21, 22, 23");
    }
}
