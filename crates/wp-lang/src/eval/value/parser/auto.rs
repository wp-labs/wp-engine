use arcstr::ArcStr;
use super::prelude::*;
use crate::ast::WplSep;
use crate::generator::FmtField;
use wp_model_core::model::DataField;

use crate::eval::runtime::field::FieldEvalUnit;
use crate::eval::value::parse_def::*;

pub struct CombinedParser {
    pub ps: Vec<ParserHold>,
}

impl CombinedParser {
    pub fn new() -> Self {
        Self { ps: Vec::new() }
    }
}
impl Default for CombinedParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldParser for CombinedParser {
    fn parse<'a>(
        &self,
        fpu: &FieldEvalUnit,
        ups_sep: &WplSep,
        data: &mut &str,
        f_name: Option<ArcStr>,
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        let mut last_e = None;

        let start = data.checkpoint();
        for p in &self.ps {
            let f_name = f_name.clone().or(Some(fpu.conf().meta_name.clone()));
            match p.parse(fpu, ups_sep, data, f_name, out) {
                Ok(o) => {
                    return Ok(o);
                }
                Err(e) => {
                    data.reset(&start);
                    last_e = Some(e);
                }
            }
        }

        if let Some(e) = last_e {
            Err(e)
        } else {
            fail.context(ctx_desc("combine parse fail"))
                .parse_next(data)
        }
    }

    fn generate(
        &self,
        _gen: &mut GenChannel,
        _ups_sep: &WplSep,
        _f_conf: &WplField,
        _g_conf: Option<&FieldGenConf>,
    ) -> AnyResult<FmtField> {
        unreachable!("Combine Parses not generate")
    }
}
