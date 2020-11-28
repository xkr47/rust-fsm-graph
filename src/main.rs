use std::env;
use std::fs::File;
use std::io::Read;
use std::process;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use syn::{Item, parse2};

use crate::parser::StateMachineDef;

mod parser;

fn main() {
    let mut args = env::args();
    let _ = args.next(); // executable name

    let filename = match (args.next(), args.next()) {
        (Some(filename), None) => filename,
        _ => {
            eprintln!("Usage: dump-syntax path/to/filename.rs");
            process::exit(1);
        }
    };

    let mut file = File::open(&filename).expect("Unable to open file");

    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");

    let syntax = syn::parse_file(&src).expect("Unable to parse file");


    // Debug impl is available if Syn is built with "extra-traits" feature.
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
        .for_each(|(name, str)| std::fs::write(format!("{}.dot", name), str).unwrap_or_else(|_| panic!("Failed to write {}.dot", name)))
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

    /*
    let mut iter = fsm.into_iter().peekable();
    let name = get_ident(iter.next().unwrap());
    let default_state = get_ident(get_single_token(get_group(iter.next().unwrap(), Delimiter::Parenthesis)));
    let mut dot = format!(r#"digraph "graph" {{
  SM_init [label="", shape=point]
  SM_init -> {}  [style="solid"]
"#, default_state);
    while let Some(first) = iter.next() {
        let from_state = get_ident(first);
        expect_punct(iter.next().unwrap(), '=', Spacing::Joint);
        expect_punct(iter.next().unwrap(), '>', Spacing::Alone);
        let stream = get_group(iter.next().unwrap(), Delimiter::Brace);
        let mut inner_iter = stream.into_iter();
        while let Some(inner) = inner_iter.next() {
            let input = get_ident(inner);
            expect_punct(inner_iter.next().unwrap(), '=', Spacing::Joint);
            expect_punct(inner_iter.next().unwrap(), '>', Spacing::Alone);


            //  "idling" -> "listening" [label="audio received"];
        }
    }
    */
    dot.push_str("}\n");
    (name, dot)
}

fn expect_punct(i: TokenTree, ch: char, spacing: Spacing) {
    if let TokenTree::Punct(p) = i {
        if p.as_char() != ch {
            panic!("Expected '{}', got '{}'", ch, p.as_char());
        }
        if p.spacing() != spacing {
            panic!("Wrong spacing after '{}'", ch);
        }
    } else {
        panic!("Expected punctuation '{}', got '{:#?}'", ch, i);
    }
}

fn get_single_token(ts: TokenStream) -> TokenTree {
    let mut iter = ts.into_iter();
    if let Some(token) = iter.next() {
        if iter.next().is_none() {
            token
        } else {
            panic!("Extra tokens in token stream");
        }
    } else {
        panic!("Empty token stream");
    }
}

fn get_group(i: TokenTree, delimiter: Delimiter) -> TokenStream {
    if let TokenTree::Group(g) = i {
        if g.delimiter() == delimiter {
            g.stream()
        } else {
            panic!("Failed to parse ident");
        }
    } else {
        panic!("Failed to parse {:?} group", delimiter);
    }
}

fn get_ident(i: TokenTree) -> String {
    if let TokenTree::Ident(s) = i {
        s.to_string()
    } else {
        panic!("Failed to parse ident");
    }
}
