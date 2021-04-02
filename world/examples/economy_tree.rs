use common::trade::Good;
use std::io::Write;
use veloren_world::site::economy::{self, good_list, Economy};
//use regex::Regex::replace_all;

fn good_name(g: Good) -> String {
    let res = format!("{:?}", g);
    let res = res.replace("(", "_");
    let res = res.replace(")", "_");
    res
}

fn labor_name(l: economy::Labor) -> String {
    let res = format!("{:?}", l);
    let res = res.replace(" ", "_");
    res
}

fn main() -> Result<(), std::io::Error> {
    let eco = Economy::default();
    let o = eco.get_orders();
    let p = eco.get_productivity();

    let mut f = std::fs::File::create("economy.gv")?;
    writeln!(f, "digraph economy {{")?;
    for i in good_list().iter() {
        let color = if economy::direct_use_goods().contains(i) {
            "green"
        } else {
            "orange"
        };
        writeln!(f, "{:?} [color=\"{}\"];", good_name(*i), color)?; // shape doubleoctagon ?
    }

    writeln!(f, "")?;
    writeln!(f, "// Professions")?;
    writeln!(f, "Everyone [shape=doubleoctagon];")?;
    for i in economy::Labor::list() {
        writeln!(f, "{:?} [shape=box];", labor_name(i))?;
    }

    writeln!(f, "")?;
    writeln!(f, "// Orders")?;
    for i in o.iter() {
        for j in i.1.iter() {
            if i.0.is_some() {
                let style = if matches!(j.0, Good::Tools)
                    || matches!(j.0, Good::Armor)
                    || matches!(j.0, Good::Potions)
                {
                    ", style=dashed, color=orange"
                } else {
                    ""
                };
                writeln!(
                    f,
                    "{:?} -> {:?} [label=\"{:.1}\"{}];",
                    good_name(j.0),
                    labor_name(i.0.unwrap()),
                    j.1,
                    style
                )?;
            } else {
                writeln!(
                    f,
                    "{:?} -> Everyone [label=\"{:.1}\"];",
                    good_name(j.0),
                    j.1
                )?;
            }
        }
    }

    writeln!(f, "")?;
    writeln!(f, "// Products")?;
    for i in p.iter() {
        for j in i.1.iter() {
            writeln!(
                f,
                "{:?} -> {:?} [label=\"{:.1}\"];",
                labor_name(i.0),
                good_name(j.0),
                j.1
            )?;
        }
    }

    writeln!(f, "}}")?;
    Ok(())
}
