use intersect_model::{
    definitions::rcore_behavior::{ir::{BinaryOp, Value, ValueType},types::Scalar},
    rcore_behavior_scalar::{binary,NumericBudget},
};
use std::io::{self,Read};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let mut input=String::new();io::stdin().read_to_string(&mut input)?;
    for line in input.lines() {
        let fields:Vec<_>=line.split('\t').collect();
        if fields.len()!=5 {return Err("Expected five verification input fields".into());}
        let kind=Scalar::named(fields[1]).ok_or("Unknown scalar kind")?;
        let op=match fields[2] {"eq"=>BinaryOp::Equal,"ne"=>BinaryOp::NotEqual,_=>return Err("Unknown operator".into())};
        let left=Value{ty:ValueType::Scalar(kind),payload:fields[3].parse()?};
        let right=Value{ty:ValueType::Scalar(kind),payload:fields[4].parse()?};
        let mut budget=NumericBudget::default();
        let result=binary(op,&left,&right,&mut budget);
        println!("{}\t{}\t{}\t{result:?}",fields[0],budget.work_used(),budget.value_bytes_used());
    }
    Ok(())
}
