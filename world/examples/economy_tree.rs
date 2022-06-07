use common::trade::Good;
use std::io::Write;
use veloren_world::site::economy::{GraphInfo, Labor};
//use regex::Regex::replace_all;

fn good_name(g: Good) -> String {
    let res = format!("{:?}", g);
    let res = res.replace('(', "_");
    res.replace(')', "_")
}

fn labor_name(l: Labor) -> String {
    let res = format!("{:?}", l);
    res.replace(' ', "_")
}

fn main() -> Result<(), std::io::Error> {
    let eco = GraphInfo::default();

    let mut f = std::fs::File::create("economy.gv")?;
    writeln!(f, "digraph economy {{")?;
    for i in eco.good_list() {
        let color = if !eco.can_store(&i) {
            "green"
        } else {
            "orange"
        };
        writeln!(f, "{:?} [color=\"{}\"];", good_name(i.into()), color)?; // shape doubleoctagon ?
    }

    writeln!(f)?;
    writeln!(f, "// Professions")?;
    writeln!(f, "Everyone [shape=doubleoctagon];")?;
    for i in eco.labor_list() {
        writeln!(f, "{:?} [shape=box];", labor_name(i))?;
    }

    writeln!(f)?;
    writeln!(f, "// Orders")?;
    let o = eco.get_orders();
    for i in o.iter() {
        for j in i.1.iter() {
            let style = if matches!(j.0.into(), Good::Tools | Good::Armor | Good::Potions) {
                ", style=dashed, color=orange"
            } else {
                ""
            };
            writeln!(
                f,
                "{:?} -> {:?} [label=\"{:.1}\"{}];",
                good_name(j.0.into()),
                labor_name(i.0),
                j.1,
                style
            )?;
        }
    }
    for j in eco.get_orders_everyone() {
        writeln!(
            f,
            "{:?} -> Everyone [label=\"{:.1}\"];",
            good_name(j.0.into()),
            j.1
        )?;
    }

    writeln!(f)?;
    writeln!(f, "// Products")?;
    let p = eco.get_production();
    for i in p.iter() {
        writeln!(
            f,
            "{:?} -> {:?} [label=\"{:.1}\"];",
            labor_name(i.0),
            good_name(i.1.0.into()),
            i.1.1
        )?;
    }

    writeln!(f, "}}")?;
    Ok(())
}
