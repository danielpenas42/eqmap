use std::rc::Rc;

use eqmap::driver::CircuitLang;
use eqmap::lut::LutLang;
use eqmap::netlist::{LogicMapper, PrimitiveCell};
use eqmap::timing::get_critical_paths;
use safety_net::graph::CombDepthInfo;
use safety_net::{DrivenNet, Netlist};
use safety_pass::CellType;

fn and_gate() -> PrimitiveCell {
    PrimitiveCell::new(CellType::AND, None)
}

fn reg_cell() -> PrimitiveCell {
    PrimitiveCell::new(CellType::FDRE, None)
}

fn timing_analysis(
    netlist: &Rc<Netlist<PrimitiveCell>>,
) -> safety_net::graph::CombDepthInfo<'_, PrimitiveCell> {
    netlist.get_analysis::<CombDepthInfo<_>>().unwrap()
}

// Visual representation
// a ──┐
//     ├── [AND left] ──┐
// b ──┘                │
//                      ├── [AND root] ── y
// c ──┐                │
//     ├── [AND right] ─┘
// d ──┘
fn reconvergent_netlist() -> (
    Rc<Netlist<PrimitiveCell>>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
) {
    let netlist = Netlist::new("reconvergent".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());

    let left = netlist
        .insert_gate(and_gate(), "left".into(), &[a, b])
        .unwrap();
    let right = netlist
        .insert_gate(and_gate(), "right".into(), &[c, d])
        .unwrap();
    let root = netlist
        .insert_gate(
            and_gate(),
            "root".into(),
            &[left.get_output(0), right.get_output(0)],
        )
        .unwrap();
    root.clone().expose_with_name("y".into());

    (
        netlist,
        root.get_output(0),
        left.get_output(0),
        right.get_output(0),
    )
}

struct TwoOutputNetlist {
    netlist: Rc<Netlist<PrimitiveCell>>,
    first_root: DrivenNet<PrimitiveCell>,
    first_leaf: DrivenNet<PrimitiveCell>,
    second_root: DrivenNet<PrimitiveCell>,
    second_leaf: DrivenNet<PrimitiveCell>,
}

fn two_output_netlist() -> TwoOutputNetlist {
    let netlist = Netlist::new("two_output".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());
    let e = netlist.insert_input("e".into());
    let f = netlist.insert_input("f".into());

    let first_leaf = netlist
        .insert_gate(and_gate(), "first_leaf".into(), &[a, b])
        .unwrap();
    let first_root = netlist
        .insert_gate(
            and_gate(),
            "first_root".into(),
            &[first_leaf.get_output(0), c],
        )
        .unwrap();
    first_root.clone().expose_with_name("y0".into());

    let second_leaf = netlist
        .insert_gate(and_gate(), "second_leaf".into(), &[d, e])
        .unwrap();
    let second_root = netlist
        .insert_gate(
            and_gate(),
            "second_root".into(),
            &[second_leaf.get_output(0), f],
        )
        .unwrap();
    second_root.clone().expose_with_name("y1".into());

    TwoOutputNetlist {
        netlist,
        first_root: first_root.get_output(0),
        first_leaf: first_leaf.get_output(0),
        second_root: second_root.get_output(0),
        second_leaf: second_leaf.get_output(0),
    }
}

fn single_chain_netlist() -> (
    Rc<Netlist<PrimitiveCell>>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
) {
    let netlist = Netlist::new("single_chain".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());

    let first = netlist
        .insert_gate(and_gate(), "first".into(), &[a, b])
        .unwrap();
    let second = netlist
        .insert_gate(and_gate(), "second".into(), &[first.get_output(0), c])
        .unwrap();
    let third = netlist
        .insert_gate(and_gate(), "third".into(), &[second.get_output(0), d])
        .unwrap();
    third.clone().expose_with_name("y".into());

    (
        netlist,
        first.get_output(0),
        second.get_output(0),
        third.get_output(0),
    )
}

#[test]
fn critical_path_uses_one_max_depth_branch() {
    let (netlist, root, left, _right) = reconvergent_netlist();
    let analysis = timing_analysis(&netlist);

    let path = get_critical_paths(&analysis, 1).into_iter().next().unwrap();

    assert_eq!(path.endpoint(), root);
    assert_eq!(path.path(), &[root, left]);
}

#[test]
fn expansion_adds_neighboring_fanin_nodes() {
    let (netlist, _root, _left, right) = reconvergent_netlist();
    let analysis = timing_analysis(&netlist);
    let path = get_critical_paths(&analysis, 1).into_iter().next().unwrap();

    let unexpanded = path.expand_n_nodes(0);
    let expanded = path.expand_n_nodes(1);

    assert!(!unexpanded.contains(&right));
    assert!(expanded.contains(&right));
}

#[test]
fn gets_multiple_critical_paths() {
    let TwoOutputNetlist {
        netlist,
        first_root,
        first_leaf,
        second_root,
        second_leaf,
    } = two_output_netlist();
    let analysis = timing_analysis(&netlist);

    let paths = get_critical_paths(&analysis, 2);

    assert_eq!(paths.len(), 2);
    assert!(
        paths
            .iter()
            .any(|path| path.path() == [first_root.clone(), first_leaf.clone()])
    );
    assert!(
        paths
            .iter()
            .any(|path| path.path() == [second_root.clone(), second_leaf.clone()])
    );
}

#[test]
fn requesting_zero_critical_paths_returns_empty_result() {
    let (netlist, _root, _left, _right) = reconvergent_netlist();
    let analysis = timing_analysis(&netlist);

    let paths = get_critical_paths(&analysis, 0);

    assert!(paths.is_empty());
}

#[test]
fn critical_paths_use_timing_endpoints_not_internal_chain_nodes() {
    let (netlist, first, second, third) = single_chain_netlist();
    let analysis = timing_analysis(&netlist);

    let paths = get_critical_paths(&analysis, 3);
    let debug_paths = paths
        .iter()
        .map(|path| {
            path.path()
                .iter()
                .map(|net| net.get_identifier().to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // if you run cargo test --test timing -- --nocapture --test-threads=1 u can see that it is printing 3 critical paths
    // when there should only be one. This is becuase it is identifying as critical ends all the ndoes in the critical path
    // this is I believe an issue with CombDepthInfo in safety net in the compute method
    eprintln!("debug critical paths: {debug_paths:?}");

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].endpoint(), third);
    assert_eq!(paths[0].depth(), 3);
    assert_eq!(paths[0].path(), &[third, second, first]);
}

#[test]
fn critical_path_stops_at_register_boundary() {
    let netlist = Netlist::new("registered".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());
    let clk = netlist.insert_input("clk".into());
    let ce = netlist.insert_input("ce".into());
    let rst = netlist.insert_input("rst".into());

    let before_a = netlist
        .insert_gate(and_gate(), "before_a".into(), &[a, b])
        .unwrap();
    let before_b = netlist
        .insert_gate(and_gate(), "before_b".into(), &[before_a.get_output(0), c])
        .unwrap();

    let reg = netlist.insert_gate_disconnected(reg_cell(), "reg".into());
    reg.find_input(&"D".into())
        .unwrap()
        .connect(before_b.get_output(0));
    reg.find_input(&"C".into()).unwrap().connect(clk);
    reg.find_input(&"CE".into()).unwrap().connect(ce);
    reg.find_input(&"R".into()).unwrap().connect(rst);

    let after = netlist
        .insert_gate(and_gate(), "after".into(), &[reg.get_output(0), d])
        .unwrap();
    after.expose_with_name("y".into());

    let analysis = timing_analysis(&netlist);
    let path = get_critical_paths(&analysis, 1).into_iter().next().unwrap();

    assert_eq!(
        path.path(),
        &[before_b.get_output(0), before_a.get_output(0)]
    );
    assert!(!path.path().contains(&reg.get_output(0)));
}

#[test]
fn insert_delay_paths_maps_only_the_critical_region() {
    let (netlist, root, left, right) = reconvergent_netlist();
    let mut mapper = netlist
        .get_analysis::<LogicMapper<LutLang, PrimitiveCell>>()
        .unwrap();

    mapper.insert_delay_paths(1, 0).unwrap();

    let mappings = mapper.mappings();
    assert_eq!(mappings.len(), 1);

    let mapping = &mappings[0];
    let roots = mapping.root_nets().collect::<Vec<_>>();
    let expr = mapping.get_expr();
    let vars = expr
        .iter()
        .filter_map(|node| node.get_var().map(|sym| sym.to_string()))
        .collect::<Vec<_>>();

    assert_eq!(roots, vec![root.clone()]);
    assert!(vars.contains(&right.get_identifier().to_string()));
    assert!(!vars.contains(&root.get_identifier().to_string()));
    assert!(!vars.contains(&left.get_identifier().to_string()));
}
