use ahash::RandomState;
use std::collections::HashMap;
use winnow::{Parser, combinator::fail};
use wp_model_core::model::{DataField, Value};
use wp_parser::WResult as ModalResult;
use wp_parser::symbol::ctx_desc;

use crate::ast::{WplFun, WplSep};

use crate::eval::runtime::group::WplEvalGroup;

pub struct FieldIndex {
    map: HashMap<String, usize, RandomState>,
}

impl FieldIndex {
    pub fn build(fields: &[DataField]) -> Self {
        let mut map: HashMap<String, usize, RandomState> =
            HashMap::with_capacity_and_hasher(fields.len(), RandomState::default());
        for (i, f) in fields.iter().enumerate() {
            map.insert(f.get_name().to_string(), i);
        }
        FieldIndex { map }
    }
    pub fn get(&self, name: &str) -> Option<usize> {
        self.map.get(name).copied()
    }
}

pub trait FiledSetProcessor {
    fn process(&self, fields: &mut Vec<DataField>, index: Option<&FieldIndex>) -> ModalResult<()>;
}
#[derive(Clone)]
pub enum PipeEnum {
    Fun(WplFun),
    Group(WplEvalGroup),
}

impl FiledSetProcessor for PipeEnum {
    #[inline]
    fn process(&self, fields: &mut Vec<DataField>, index: Option<&FieldIndex>) -> ModalResult<()> {
        match self {
            PipeEnum::Fun(pipe) => return pipe.process(fields, index),
            PipeEnum::Group(pipe) => {
                let len = fields.len();
                if len >= 1 {
                    let res_data = fields.remove(len - 1);
                    if let Value::Chars(res_data) = res_data.get_value() {
                        let sep = WplSep::default();
                        let mut data = res_data.as_str();
                        pipe.proc(&sep, &mut data, fields)?;
                        return Ok(());
                    }
                } else {
                    //pipe pass
                    return Ok(());
                }
            }
        };
        fail.context(ctx_desc("not support parse pipe"))
            .parse_next(&mut "")
    }
}
