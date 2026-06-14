use crate::vm::value::Value;
use std::collections::BTreeSet;

pub fn set_op(
    a: &BTreeSet<Value>,
    b: &BTreeSet<Value>,
    op: u8,
) -> BTreeSet<Value> {
    match op {
        0 => { 
            let mut r = a.clone(); 
            for v in b { unsafe { v.inc_ref(); } r.insert(*v); }
            r 
        }
        1 => a.iter().filter(|x| b.contains(x)).map(|x| { unsafe { x.inc_ref(); } *x }).collect(),
        2 => a.iter().filter(|x| !b.contains(x)).map(|x| { unsafe { x.inc_ref(); } *x }).collect(),
        _ => {
            let mut res = BTreeSet::new();
            for x in a.iter().filter(|x| !b.contains(x)) { unsafe { x.inc_ref(); } res.insert(*x); }
            for x in b.iter().filter(|x| !a.contains(x)) { unsafe { x.inc_ref(); } res.insert(*x); }
            res
        }
    }
}
